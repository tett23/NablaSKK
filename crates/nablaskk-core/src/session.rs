// Ported from AquaSKK (https://github.com/codefirst/aquaskk):
//   src/engine/state/ (SKKState, SKKStatePrimary-inl,
//   SKKStateComposing-inl, SKKStateRecursiveRegister-inl,
//   SKKStateEntryRemove-inl)
//   src/engine/session/ (SKKInputSession, SKKRecursiveEditor,
//   SKKInputEnvironment, SKKInputModeSelector)
// Copyright (C) 2008 Tomotaka SUWA <t.suwa@mac.com>
// Ported to Rust by tett23, 2026.
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// any later version. See the LICENSE file for details.

//! Input session: the SKK state machine and the nested-registration
//! editor stack (port of the `state` and `session` directories).

use crate::backend::Backend;
use crate::bridge::{Annotator, CandidateWindow, Clipboard, DynamicCompletor, FrontEnd, Messenger};
use crate::completer::Completer;
use crate::config::Config;
use crate::context::{InputContext, RegistrationState, UndoResult};
use crate::editor::{BottomEditor, EngineDeps, InputEngine, PrimaryEditor, RegisterEditor};
use crate::event::{Event, EventId, HandleOption};
use crate::input_mode::InputMode;
use crate::machine::{StateHandler, StateMachine, StateResult, SystemEvent};
use crate::selector::Selector;
use crate::trie::RomanKanaConverter;

// ======================================================================
// State ids (port of the SKKState handler hierarchy)
// ======================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateId {
    Top,
    Primary,
    KanaInput,
    Hirakana,
    Katakana,
    Jisx0201Kana,
    LatinInput,
    Ascii,
    Jisx0208Latin,
    Composing,
    Edit,
    EntryInput,
    KanaEntry,
    AsciiEntry,
    EntryCompletion,
    SelectCandidate,
    OkuriInput,
    EntryRemove,
    RecursiveRegister,
}

// ======================================================================
// One nested editor level (port of SKKRecursiveEditor)
// ======================================================================

struct Level {
    engine: InputEngine,
    machine: StateMachine<StateId>,
    selector: Selector,
    completer: Completer,
}

impl Level {
    fn new(bottom: BottomEditor) -> Self {
        Self {
            engine: InputEngine::new(bottom),
            machine: StateMachine::new(StateId::Top),
            selector: Selector::new(),
            completer: Completer::new(),
        }
    }
}

// ======================================================================
// State handler wiring
// ======================================================================

struct States<'a> {
    engine: &'a mut InputEngine,
    selector: &'a mut Selector,
    completer: &'a mut Completer,
    ctx: &'a mut InputContext,
    backend: &'a mut Backend,
    converter: &'a RomanKanaConverter,
    config: &'a Config,
    frontend: &'a mut dyn FrontEnd,
    window: &'a mut dyn CandidateWindow,
    messenger: &'a mut dyn Messenger,
    clipboard: &'a mut dyn Clipboard,
}

use StateId::*;
use StateResult::{
    DeepForward, DeepHistory, Forward, Handled, Initial, SaveHistory, ShallowHistory, Super,
    Transition,
};

type R = StateResult<StateId>;

impl States<'_> {
    fn with_deps<T>(&mut self, f: impl FnOnce(&mut InputEngine, &mut InputContext, &mut EngineDeps) -> T) -> T {
        let mut deps =
            EngineDeps { backend: self.backend, converter: self.converter, config: self.config };
        f(self.engine, self.ctx, &mut deps)
    }

    fn commit(&mut self) {
        self.with_deps(|engine, ctx, deps| engine.commit(ctx, deps));
    }

    fn can_convert(&self, code: u8) -> bool {
        self.engine.can_convert(self.converter, code)
    }

    fn handle_char(&mut self, code: u8, direct: bool) {
        self.with_deps(|engine, ctx, deps| engine.handle_char(ctx, deps, code, direct));
    }

    /// Egg-like-newline handling shared by commit-on-enter states.
    fn commit_transition(&mut self, event: &Event) -> R {
        self.commit();

        if event.id == EventId::Enter && !self.config.suppress_newline_on_commit {
            Forward(KanaInput)
        } else {
            Transition(KanaInput)
        }
    }

    /// Port of SKKSelector::Execute + buddy notify.
    fn selector_execute(&mut self) -> bool {
        let entry =
            self.with_deps(|engine, ctx, deps| engine.selector_query_entry(ctx, deps));

        let found = self.selector.execute(
            self.backend,
            self.window,
            &entry,
            self.config.max_count_of_inline_candidates,
        );

        if found {
            self.sync_candidate();
        }

        found
    }

    fn sync_candidate(&mut self) {
        if let Some(candidate) = self.selector.current().cloned() {
            self.engine.set_candidate(self.ctx, &candidate);
        }
    }

    fn selector_next(&mut self) -> bool {
        let moved = self.selector.next(self.window);
        if moved {
            self.sync_candidate();
        }
        moved
    }

    fn selector_prev(&mut self) -> bool {
        let moved = self.selector.prev(self.window);
        if moved {
            self.sync_candidate();
        }
        moved
    }

    /// Port of SKKCompleter::Execute + buddy notify.
    fn completer_execute(&mut self, limit: usize) -> bool {
        let query =
            self.with_deps(|engine, ctx, deps| engine.completer_query_string(ctx, deps));

        let found = self.completer.execute(self.backend, &query, limit);
        if found {
            self.sync_completion();
        }

        found
    }

    fn sync_completion(&mut self) {
        if let Some(entry) = self.completer.current().map(str::to_string) {
            self.engine.set_composing_entry(self.ctx, &entry);
        }
    }

    // ==================================================================
    // State handlers
    // ==================================================================

    fn top(&mut self, _event: &Event) -> R {
        Handled
    }

    fn primary(&mut self, event: &Event) -> R {
        match event.id {
            EventId::JMode => {
                self.commit();
                Handled
            }
            EventId::Enter => {
                self.with_deps(|engine, ctx, deps| engine.handle_enter(ctx, deps));
                Handled
            }
            EventId::Cancel => {
                self.with_deps(|engine, ctx, deps| engine.handle_cancel(ctx, deps));
                Handled
            }
            EventId::Undo => {
                match self.ctx.undo.undo(self.frontend, self.backend) {
                    UndoResult::KanaEntry => Transition(KanaEntry),
                    UndoResult::AsciiEntry => Transition(AsciiEntry),
                    UndoResult::Failed => {
                        self.messenger.send_message("Undo できませんでした");
                        self.ctx.event_handled = false;
                        Handled
                    }
                }
            }
            EventId::Paste => {
                let text = self.clipboard.paste_string();
                self.engine.handle_paste(self.ctx, &text);
                Handled
            }
            EventId::Ping => Handled,
            EventId::Backspace => {
                self.engine.handle_backspace(self.ctx);
                Handled
            }
            EventId::Delete => {
                self.engine.handle_delete(self.ctx);
                Handled
            }
            EventId::Left => {
                self.engine.handle_cursor_left(self.ctx);
                Handled
            }
            EventId::Right => {
                self.engine.handle_cursor_right(self.ctx);
                Handled
            }
            EventId::Up => {
                self.engine.handle_cursor_up(self.ctx);
                Handled
            }
            EventId::Down => {
                self.engine.handle_cursor_down(self.ctx);
                Handled
            }
            EventId::AsciiMode => Transition(Ascii),
            EventId::HirakanaMode => Transition(Hirakana),
            EventId::KatakanaMode => Transition(Katakana),
            EventId::Jisx0201KanaMode => Transition(Jisx0201Kana),
            EventId::Jisx0208LatinMode => Transition(Jisx0208Latin),
            _ => {
                // Unhandled events reset the engine and pass through
                self.with_deps(|engine, ctx, deps| engine.reset(ctx, deps));
                Handled
            }
        }
    }

    fn kana_input(&mut self, event: &Event) -> R {
        if event.id == EventId::Char {
            if !self.can_convert(event.code) {
                if event.is_switch_to_ascii() {
                    return Transition(Ascii);
                }
                if event.is_switch_to_jisx0208_latin() {
                    return Transition(Jisx0208Latin);
                }
                if event.is_enter_abbrev() {
                    return Transition(AsciiEntry);
                }
                if event.is_enter_japanese() {
                    return Transition(KanaEntry);
                }
            }

            if event.is_sticky_key() {
                return Transition(KanaEntry);
            }
            if event.is_upper_cases() {
                return Forward(KanaEntry);
            }

            if event.is_input_chars() {
                self.handle_char(event.code, event.is_direct());
                return Handled;
            }
        }

        Super(Primary)
    }

    fn hirakana(&mut self, event: &Event) -> R {
        match event.id {
            EventId::HirakanaMode => Handled,
            EventId::Char if !(event.is_input_chars() && self.can_convert(event.code)) => {
                if event.is_toggle_kana() {
                    return Transition(Katakana);
                }
                if event.is_toggle_jisx0201_kana() {
                    return Transition(Jisx0201Kana);
                }
                Super(KanaInput)
            }
            _ => Super(KanaInput),
        }
    }

    fn katakana(&mut self, event: &Event) -> R {
        match event.id {
            EventId::KatakanaMode => Handled,
            _ => {
                let converting = event.id == EventId::Char
                    && event.is_input_chars()
                    && self.can_convert(event.code);

                if !converting {
                    if event.id == EventId::JMode || event.is_toggle_kana() {
                        return Transition(Hirakana);
                    }
                    if event.is_toggle_jisx0201_kana() {
                        return Transition(Jisx0201Kana);
                    }
                }

                Super(KanaInput)
            }
        }
    }

    fn jisx0201_kana(&mut self, event: &Event) -> R {
        match event.id {
            EventId::Jisx0201KanaMode => Handled,
            _ => {
                let converting = event.id == EventId::Char
                    && event.is_input_chars()
                    && self.can_convert(event.code);

                if !converting
                    && (event.id == EventId::JMode
                        || event.is_toggle_kana()
                        || event.is_toggle_jisx0201_kana())
                {
                    return Transition(Hirakana);
                }

                Super(KanaInput)
            }
        }
    }

    fn latin_input(&mut self, event: &Event) -> R {
        match event.id {
            EventId::JMode => Transition(Hirakana),
            EventId::Char if event.is_input_chars() => {
                let code = if event.option == HandleOption::CapsLock {
                    event.code.to_ascii_uppercase()
                } else {
                    event.code
                };
                self.handle_char(code, event.is_direct());
                Handled
            }
            _ => Super(Primary),
        }
    }

    fn ascii(&mut self, event: &Event) -> R {
        match event.id {
            EventId::AsciiMode => Handled,
            _ => Super(LatinInput),
        }
    }

    fn jisx0208_latin(&mut self, event: &Event) -> R {
        match event.id {
            EventId::Jisx0208LatinMode => Handled,
            EventId::AsciiMode => Transition(Ascii),
            _ => {
                if !event.is_input_chars() && event.is_switch_to_ascii() {
                    return Transition(Ascii);
                }
                Super(LatinInput)
            }
        }
    }

    fn composing(&mut self, event: &Event) -> R {
        match event.id {
            EventId::Ping => Handled,
            _ => Super(Top),
        }
    }

    fn edit(&mut self, event: &Event) -> R {
        match event.id {
            EventId::Enter | EventId::JMode => self.commit_transition(event),
            EventId::Cancel => {
                if self.ctx.undo.is_active() {
                    let candidate = self.ctx.undo.candidate().to_string();
                    self.ctx.output.fix(&candidate);
                }
                Transition(KanaInput)
            }
            EventId::Backspace => {
                self.engine.handle_backspace(self.ctx);
                if self.ctx.needs_setback {
                    Transition(KanaInput)
                } else {
                    Handled
                }
            }
            EventId::Delete => {
                self.engine.handle_delete(self.ctx);
                Handled
            }
            EventId::Left => {
                self.engine.handle_cursor_left(self.ctx);
                Handled
            }
            EventId::Right => {
                self.engine.handle_cursor_right(self.ctx);
                Handled
            }
            EventId::Up => {
                self.engine.handle_cursor_up(self.ctx);
                Handled
            }
            EventId::Down => {
                self.engine.handle_cursor_down(self.ctx);
                Handled
            }
            EventId::Char => {
                if event.is_comp_conversion() {
                    self.completer_execute(1);
                }

                if event.is_next_candidate() || event.is_comp_conversion() {
                    if self.ctx.entry.is_empty() {
                        return Transition(KanaInput);
                    }

                    if self.selector_execute() {
                        return Transition(SelectCandidate);
                    }

                    return Transition(RecursiveRegister);
                }

                Handled
            }
            _ => Super(Composing),
        }
    }

    fn entry_input(&mut self, event: &Event) -> R {
        match event.id {
            EventId::Tab => {
                if self.completer_execute(0) {
                    Transition(EntryCompletion)
                } else {
                    Handled
                }
            }
            _ => Super(Edit),
        }
    }

    fn kana_entry(&mut self, event: &Event) -> R {
        if event.id == EventId::Char {
            if event.is_next_candidate() {
                return Super(EntryInput);
            }

            if event.is_toggle_kana() {
                self.with_deps(|engine, ctx, deps| engine.toggle_kana(ctx, deps));
                return Transition(KanaInput);
            }

            if event.is_toggle_jisx0201_kana() {
                self.with_deps(|engine, ctx, deps| engine.toggle_jisx0201_kana(ctx, deps));
                return Transition(KanaInput);
            }

            if event.is_sticky_key() {
                if self.ctx.entry.is_empty() {
                    if event.is_input_chars() {
                        self.handle_char(event.code, event.is_direct());
                    }
                    self.commit();
                    return Transition(KanaInput);
                } else {
                    return Transition(OkuriInput);
                }
            }

            if event.is_upper_cases() && !self.ctx.entry.is_empty() {
                return Forward(OkuriInput);
            }

            if !self.can_convert(event.code) {
                if event.is_switch_to_ascii() {
                    self.commit();
                    return Transition(Ascii);
                }

                if event.is_switch_to_jisx0208_latin() {
                    self.commit();
                    return Transition(Jisx0208Latin);
                }

                if event.is_enter_japanese() {
                    if self.config.handle_recursive_entry_as_okuri && !self.ctx.entry.is_empty() {
                        return Transition(OkuriInput);
                    }

                    self.commit();
                    return Forward(KanaInput);
                }
            }

            if event.is_input_chars() {
                self.handle_char(event.code, event.is_direct());
                return Handled;
            }
        }

        Super(EntryInput)
    }

    fn ascii_entry(&mut self, event: &Event) -> R {
        if event.id == EventId::Char {
            if event.is_next_candidate() {
                return Super(EntryInput);
            }

            if event.is_toggle_jisx0201_kana() && !self.ctx.entry.is_empty() {
                self.with_deps(|engine, ctx, deps| engine.toggle_jisx0201_kana(ctx, deps));
                return Transition(KanaInput);
            }

            if event.is_input_chars() {
                self.handle_char(event.code, event.is_direct());
                return Handled;
            }
        }

        Super(EntryInput)
    }

    fn entry_completion(&mut self, event: &Event) -> R {
        if event.id == EventId::Tab || event.id == EventId::Char {
            if event.id == EventId::Tab || event.is_next_completion() {
                self.completer.next();
                self.sync_completion();
                return Handled;
            }

            if event.is_prev_completion() {
                self.completer.prev();
                self.sync_completion();
                return Handled;
            }

            if event.is_next_candidate() {
                return Super(Edit);
            }

            if event.is_remove_trigger() {
                if self.completer.remove(self.backend) {
                    self.messenger.send_message("見出し語を削除しました");
                    return Transition(KanaInput);
                } else {
                    return Handled;
                }
            }
        }

        // Everything else replays into the previous entry state
        DeepForward(EntryInput)
    }

    fn select_candidate(&mut self, event: &Event) -> R {
        match event.id {
            EventId::Enter | EventId::JMode => self.commit_transition(event),
            EventId::Cancel => DeepHistory(EntryInput),
            EventId::Left => {
                self.selector.cursor_left(self.window);
                self.sync_candidate();
                Handled
            }
            EventId::Right => {
                self.selector.cursor_right(self.window);
                self.sync_candidate();
                Handled
            }
            EventId::Up => {
                self.selector.cursor_up(self.window);
                self.sync_candidate();
                Handled
            }
            EventId::Down => {
                self.selector.cursor_down(self.window);
                self.sync_candidate();
                Handled
            }
            EventId::Backspace | EventId::Char => {
                if event.id == EventId::Backspace
                    && self.selector.is_inline()
                    && self.config.inline_backspace_implies_commit
                {
                    self.commit();
                    return Forward(KanaInput);
                }

                if event.id == EventId::Backspace || event.is_prev_candidate() {
                    if !self.selector_prev() {
                        return DeepHistory(EntryInput);
                    }
                    return Handled;
                }

                if event.is_next_candidate() {
                    if !self.selector_next() {
                        return Transition(RecursiveRegister);
                    }
                    return Handled;
                }

                if event.is_remove_trigger() {
                    return Transition(EntryRemove);
                }

                if event.is_input_chars() || event.is_toggle_jisx0201_kana() {
                    if self.selector.is_inline() {
                        self.commit();
                        return DeepForward(KanaInput);
                    }

                    if self.selector.select(self.window, event.code) {
                        self.sync_candidate();
                        self.commit();
                        return Transition(KanaInput);
                    }
                }

                Handled
            }
            _ => Super(Composing),
        }
    }

    fn okuri_input(&mut self, event: &Event) -> R {
        match event.id {
            EventId::Enter | EventId::JMode => self.commit_transition(event),
            EventId::Cancel => {
                self.with_deps(|engine, ctx, deps| engine.reset(ctx, deps));
                Transition(KanaEntry)
            }
            EventId::Tab
            | EventId::Delete
            | EventId::Left
            | EventId::Right
            | EventId::Up
            | EventId::Down => Handled,
            EventId::Backspace => {
                self.engine.handle_backspace(self.ctx);

                if self.ctx.needs_setback {
                    Transition(KanaEntry)
                } else {
                    Handled
                }
            }
            EventId::Char => {
                if event.is_input_chars() {
                    self.handle_char(event.code, event.is_direct());
                }

                if event.is_next_candidate() || self.engine.is_okuri_complete() {
                    if self.selector_execute() {
                        return Transition(SelectCandidate);
                    }

                    return Transition(RecursiveRegister);
                }

                Handled
            }
            _ => Super(Top),
        }
    }

    fn entry_remove(&mut self, event: &Event) -> R {
        match event.id {
            EventId::Enter => {
                self.commit();

                if !self.ctx.needs_setback {
                    self.messenger.send_message("単語を削除しました");
                    return Transition(KanaInput);
                }

                Transition(SelectCandidate)
            }
            EventId::Cancel => Transition(SelectCandidate),
            EventId::Backspace => {
                self.engine.handle_backspace(self.ctx);
                Handled
            }
            EventId::Char if event.is_input_chars() => {
                // Removal confirmation is typed in raw ASCII
                self.handle_char(event.code, true);
                Handled
            }
            _ => Super(Top),
        }
    }

    fn recursive_register(&mut self, event: &Event) -> R {
        match event.id {
            EventId::Enter => Transition(KanaInput),
            EventId::Cancel => DeepHistory(Composing),
            _ => Super(Top),
        }
    }
}

impl StateHandler for States<'_> {
    type Id = StateId;
    type Event = Event;

    fn initial_state(&self) -> StateId {
        Primary
    }

    fn handle(&mut self, state: StateId, event: &Event) -> R {
        match state {
            Top => self.top(event),
            Primary => self.primary(event),
            KanaInput => self.kana_input(event),
            Hirakana => self.hirakana(event),
            Katakana => self.katakana(event),
            Jisx0201Kana => self.jisx0201_kana(event),
            LatinInput => self.latin_input(event),
            Ascii => self.ascii(event),
            Jisx0208Latin => self.jisx0208_latin(event),
            Composing => self.composing(event),
            Edit => self.edit(event),
            EntryInput => self.entry_input(event),
            KanaEntry => self.kana_entry(event),
            AsciiEntry => self.ascii_entry(event),
            EntryCompletion => self.entry_completion(event),
            SelectCandidate => self.select_candidate(event),
            OkuriInput => self.okuri_input(event),
            EntryRemove => self.entry_remove(event),
            RecursiveRegister => self.recursive_register(event),
        }
    }

    fn handle_system(&mut self, state: StateId, event: SystemEvent) -> R {
        use SystemEvent::*;

        match (state, event) {
            (Top, Probe) => StateResult::Top,
            (Top, Init) => Initial(self.initial_state()),

            (Primary, Probe) => Super(Top),
            (Primary, Init) => Initial(KanaInput),
            (Primary, Entry) => {
                self.with_deps(|engine, ctx, deps| engine.set_state_primary(ctx, deps));
                Handled
            }

            (KanaInput, Probe) => Super(Primary),
            (KanaInput, Init) => ShallowHistory(Hirakana),
            (KanaInput, Exit) => SaveHistory,

            (Hirakana, Probe) => Super(KanaInput),
            (Hirakana, Entry) => {
                self.engine.select_input_mode(self.ctx, InputMode::Hirakana);
                Handled
            }

            (Katakana, Probe) => Super(KanaInput),
            (Katakana, Entry) => {
                self.engine.select_input_mode(self.ctx, InputMode::Katakana);
                Handled
            }

            (Jisx0201Kana, Probe) => Super(KanaInput),
            (Jisx0201Kana, Entry) => {
                self.engine.select_input_mode(self.ctx, InputMode::Jisx0201Kana);
                Handled
            }

            (LatinInput, Probe) => Super(Primary),

            (Ascii, Probe) => Super(LatinInput),
            (Ascii, Entry) => {
                self.engine.select_input_mode(self.ctx, InputMode::Ascii);
                Handled
            }

            (Jisx0208Latin, Probe) => Super(LatinInput),
            (Jisx0208Latin, Entry) => {
                self.engine.select_input_mode(self.ctx, InputMode::Jisx0208Latin);
                Handled
            }

            (Composing, Probe) => Super(Top),
            (Composing, Exit) => SaveHistory,

            (Edit, Probe) => Super(Composing),
            (Edit, Exit) => {
                self.ctx.undo.clear();
                SaveHistory
            }

            (EntryInput, Probe) => Super(Edit),
            (EntryInput, Entry) => {
                self.with_deps(|engine, ctx, deps| engine.set_state_composing(ctx, deps));
                Handled
            }
            (EntryInput, Exit) => SaveHistory,

            (KanaEntry, Probe) => Super(EntryInput),
            (KanaEntry, Entry) => {
                // Re-entry support
                self.with_deps(|engine, ctx, deps| engine.set_state_composing(ctx, deps));
                Handled
            }

            (AsciiEntry, Probe) => Super(EntryInput),
            (AsciiEntry, Entry) => {
                self.engine.select_input_mode(self.ctx, InputMode::Ascii);
                Handled
            }

            (EntryCompletion, Probe) => Super(Edit),
            (EntryCompletion, Entry) => {
                self.with_deps(|engine, ctx, deps| engine.set_state_composing(ctx, deps));
                Handled
            }

            (SelectCandidate, Probe) => Super(Composing),
            (SelectCandidate, Entry) => {
                self.with_deps(|engine, ctx, deps| engine.set_state_select_candidate(ctx, deps));
                self.selector.show(self.window);
                Handled
            }
            (SelectCandidate, Exit) => {
                self.selector.hide(self.window);
                Handled
            }

            (OkuriInput, Probe) => Super(Top),
            (OkuriInput, Entry) => {
                self.with_deps(|engine, ctx, deps| engine.set_state_okuri(ctx, deps));
                Handled
            }

            (EntryRemove, Probe) => Super(Top),
            (EntryRemove, Entry) => {
                self.with_deps(|engine, ctx, deps| engine.set_state_entry_remove(ctx, deps));
                Handled
            }

            (RecursiveRegister, Probe) => Super(Top),
            (RecursiveRegister, Entry) => {
                self.with_deps(|engine, ctx, deps| engine.set_state_registration(ctx, deps));
                self.messenger.beep();
                Handled
            }

            _ => Handled,
        }
    }
}

// ======================================================================
// Session parameters
// ======================================================================

/// External resources for a session (port of `SKKInputSessionParameter`).
pub struct SessionParameter {
    pub backend: Backend,
    pub converter: RomanKanaConverter,
    pub config: Config,
    pub frontend: Box<dyn FrontEnd>,
    pub window: Box<dyn CandidateWindow>,
    pub messenger: Box<dyn Messenger>,
    pub clipboard: Box<dyn Clipboard>,
    pub annotator: Box<dyn Annotator>,
    pub completor: Box<dyn DynamicCompletor>,
}

// ======================================================================
// Input session (port of SKKInputSession)
// ======================================================================

pub struct Session {
    param: SessionParameter,
    context: InputContext,
    stack: Vec<Level>,
    in_event: bool,
}

impl Session {
    pub fn new(param: SessionParameter) -> Self {
        let mut session = Self {
            param,
            context: InputContext::default(),
            stack: Vec::new(),
            in_event: false,
        };

        let level = session.create_level(BottomEditor::Primary(PrimaryEditor));
        session.stack.push(level);
        session
    }

    fn create_level(&mut self, bottom: BottomEditor) -> Level {
        let mut level = Level::new(bottom);

        let mut deps = EngineDeps {
            backend: &mut self.param.backend,
            converter: &self.param.converter,
            config: &self.param.config,
        };
        level.engine.set_state_primary(&mut self.context, &mut deps);

        level
    }

    fn dispatch_top(&mut self, event: &Event) {
        let level = self.stack.last_mut().expect("session stack is empty");

        let mut states = States {
            engine: &mut level.engine,
            selector: &mut level.selector,
            completer: &mut level.completer,
            ctx: &mut self.context,
            backend: &mut self.param.backend,
            converter: &self.param.converter,
            config: &self.param.config,
            frontend: self.param.frontend.as_mut(),
            window: self.param.window.as_mut(),
            messenger: self.param.messenger.as_mut(),
            clipboard: self.param.clipboard.as_mut(),
        };

        level.machine.dispatch(&mut states, event);
    }

    /// Process one input event. Returns true when the event was consumed.
    pub fn handle_event(&mut self, event: &Event) -> bool {
        if self.in_event {
            return false;
        }
        self.in_event = true;

        self.begin_event();
        self.dispatch_top(event);
        self.end_event();
        self.output();

        self.in_event = false;
        self.result(event)
    }

    /// Commit any pending text.
    pub fn commit(&mut self) {
        self.handle_event(&Event::new(EventId::Enter, 0, 0));

        if self.context.output.is_composing() {
            self.clear();
        }
    }

    /// Reset to the initial state.
    pub fn clear(&mut self) {
        if self.in_event {
            return;
        }
        self.in_event = true;

        self.stack.clear();
        self.context = InputContext::default();
        let level = self.create_level(BottomEditor::Primary(PrimaryEditor));
        self.stack.push(level);

        self.output();
        self.in_event = false;
    }

    /// The current input mode of the active level.
    pub fn input_mode(&self) -> InputMode {
        self.stack.last().map(|level| level.engine.input_mode()).unwrap_or_default()
    }

    /// The composing string currently displayed.
    pub fn composing_string(&self) -> &str {
        self.context.output.composing_string()
    }

    pub fn backend(&self) -> &Backend {
        &self.param.backend
    }

    pub fn converter_mut(&mut self) -> &mut RomanKanaConverter {
        &mut self.param.converter
    }

    pub fn config(&self) -> &Config {
        &self.param.config
    }

    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.param.config
    }

    pub fn backend_mut(&mut self) -> &mut Backend {
        &mut self.param.backend
    }

    // ------------------------------------------------------------

    fn begin_event(&mut self) {
        self.context.event_handled = true;
        self.context.needs_setback = false;
    }

    fn end_event(&mut self) {
        match self.context.registration.state() {
            RegistrationState::Started => {
                self.context.registration.clear();
                let entry = self.context.entry.clone();
                let level = self.create_level(BottomEditor::Register(RegisterEditor::new(&entry)));
                self.stack.push(level);
            }
            state @ (RegistrationState::Finished | RegistrationState::Aborted) => {
                if self.stack.len() != 1 {
                    self.stack.pop();

                    let id = if state == RegistrationState::Finished {
                        EventId::Enter
                    } else {
                        EventId::Cancel
                    };
                    self.dispatch_top(&Event::new(id, 0, 0));
                }
            }
            RegistrationState::None => {}
        }
    }

    /// Port of SKKRecursiveEditor::Output.
    fn output(&mut self) {
        let level = self.stack.last_mut().expect("session stack is empty");

        level.engine.update_input_context(&mut self.context, &self.param.config);
        self.context.output.output(self.param.frontend.as_mut());

        // Dynamic completion
        if self.context.dynamic_completion && self.param.config.enable_dynamic_completion {
            let entry = self.context.entry.clone();
            let mut joined = String::new();
            let mut common_prefix = String::new();

            if !entry.is_empty() && !entry.is_okuri_ari() {
                let range = self.param.config.dynamic_completion_range;

                if range > 0 {
                    let result = self.param.backend.complete(entry.entry_string(), range);

                    if !result.is_empty() {
                        common_prefix = result[0].clone();
                        for completion in &result {
                            common_prefix = common_prefix_of(&common_prefix, completion);
                            joined += completion;
                            joined.push('\n');
                        }
                        joined.pop();
                    }
                }
            }

            let prefix_len = common_prefix.chars().count();
            let mark = self.context.output.mark();
            self.param.completor.update(&joined, prefix_len, mark);
            self.param.completor.show();
        } else {
            self.param.completor.hide();
        }

        // Annotation
        if self.context.annotation && self.param.config.enable_annotation {
            let candidate = self.context.candidate.clone();
            let mark = self.context.output.mark();
            self.param.annotator.update(&candidate, mark);
            self.param.annotator.show();
        } else {
            self.param.annotator.hide();
        }
    }

    fn result(&self, event: &Event) -> bool {
        // Always handled while registering or composing
        if self.stack.len() != 1 || self.context.output.is_composing() {
            return true;
        }

        match event.option {
            HandleOption::AlwaysHandled => true,
            HandleOption::PseudoHandled => false,
            _ => self.context.event_handled,
        }
    }
}

fn common_prefix_of(a: &str, b: &str) -> String {
    a.chars()
        .zip(b.chars())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x)
        .collect()
}
