//! Editor stack (port of the `editor` directory).

use crate::backend::Backend;
use crate::candidate::Candidate;
use crate::config::Config;
use crate::context::{InputContext, RegistrationState};
use crate::entry::Entry;
use crate::input_mode::InputMode;
use crate::jconv;
use crate::trie::RomanKanaConverter;

// ======================================================================
// Text buffer with cursor support (port of SKKTextBuffer)
// ======================================================================

/// Character-based text buffer. The cursor is a non-positive offset from
/// the end (0 = end of text).
#[derive(Debug, Clone, Default)]
pub struct TextBuffer {
    chars: Vec<char>,
    cursor: i32,
}

impl TextBuffer {
    fn insert_pos(&self) -> usize {
        (self.chars.len() as i32 + self.cursor).max(0) as usize
    }

    fn min_cursor(&self) -> i32 {
        -(self.chars.len() as i32)
    }

    pub fn insert(&mut self, str_: &str) {
        let pos = self.insert_pos();
        for (index, c) in str_.chars().enumerate() {
            self.chars.insert(pos + index, c);
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor != self.min_cursor() {
            let pos = self.insert_pos();
            self.chars.remove(pos - 1);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor != 0 {
            self.cursor_right();
            self.backspace();
        }
    }

    pub fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }

    pub fn cursor_left(&mut self) {
        if self.cursor != self.min_cursor() {
            self.cursor -= 1;
        }
    }

    pub fn cursor_right(&mut self) {
        if self.cursor != 0 {
            self.cursor += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        self.cursor = self.min_cursor();
    }

    pub fn cursor_down(&mut self) {
        self.cursor = 0;
    }

    pub fn cursor_position(&self) -> i32 {
        self.cursor
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn string(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn left_string(&self) -> String {
        self.chars[..self.insert_pos()].iter().collect()
    }

    pub fn right_string(&self) -> String {
        self.chars[self.insert_pos()..].iter().collect()
    }
}

// ======================================================================
// Romaji input queue (port of SKKInputQueue)
// ======================================================================

/// One conversion step's outcome, routed to the active editor
/// (port of `SKKInputQueueObserver::State`).
#[derive(Debug, Clone, Default)]
pub struct QueueState {
    pub fixed: String,
    pub intermediate: String,
    pub queue: String,
    pub code: u8,
}

#[derive(Debug, Clone, Default)]
pub struct InputQueue {
    mode: InputMode,
    queue: String,
}

impl InputQueue {
    pub fn select_input_mode(&mut self, mode: InputMode) -> QueueState {
        self.mode = mode;
        self.clear()
    }

    pub fn add_char(&mut self, converter: &RomanKanaConverter, code: u8, direct: bool) -> QueueState {
        let mut state = QueueState { code, ..QueueState::default() };

        if direct || self.mode == InputMode::Ascii {
            state.fixed.push(code as char);
        } else {
            match self.mode {
                InputMode::Hirakana | InputMode::Katakana | InputMode::Jisx0201Kana =>
                {
                    self.queue.push(code.to_ascii_lowercase() as char);
                    let (result, _) = converter.convert(self.mode, &self.queue);
                    self.queue = result.next.clone();
                    state.fixed = result.output;
                    state.intermediate = result.intermediate;
                }
                InputMode::Jisx0208Latin => {
                    self.queue.push(code as char);
                    state.fixed = jconv::ascii_to_jisx0208_latin(&self.queue);
                    self.queue.clear();
                }
                InputMode::Ascii => unreachable!(),
            }
        }

        state.queue = self.queue.clone();
        state
    }

    pub fn remove_char(&mut self) -> Option<QueueState> {
        if self.is_empty() {
            return None;
        }

        self.queue.pop();

        Some(QueueState { queue: self.queue.clone(), ..QueueState::default() })
    }

    /// Fix the intermediate state (n -> ん).
    pub fn terminate(&mut self, converter: &RomanKanaConverter) -> Option<QueueState> {
        if self.is_empty() {
            return None;
        }

        let mut state = QueueState::default();

        match self.mode {
            InputMode::Hirakana | InputMode::Katakana | InputMode::Jisx0201Kana => {
                let (result, _) = converter.convert(self.mode, &self.queue);
                state.fixed = result.output + &result.intermediate;
            }
            _ => {}
        }

        self.queue.clear();

        Some(state)
    }

    pub fn clear(&mut self) -> QueueState {
        self.queue.clear();
        QueueState::default()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn queue_string(&self) -> &str {
        &self.queue
    }

    pub fn can_convert(&self, converter: &RomanKanaConverter, code: u8) -> bool {
        match self.mode {
            InputMode::Hirakana | InputMode::Katakana | InputMode::Jisx0201Kana => {
                let mut tmp = self.queue.clone();
                tmp.push(code.to_ascii_lowercase() as char);
                let (_, converted) = converter.convert(self.mode, &tmp);
                converted
            }
            _ => false,
        }
    }
}

// ======================================================================
// Editor events
// ======================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorEvent {
    BackSpace,
    Delete,
    CursorLeft,
    CursorRight,
    CursorUp,
    CursorDown,
}

// ======================================================================
// Concrete editors
// ======================================================================

/// Direct input pass-through (port of SKKPrimaryEditor).
#[derive(Debug, Clone, Default)]
pub struct PrimaryEditor;

impl PrimaryEditor {
    fn read_context(&mut self, ctx: &mut InputContext) {
        ctx.entry = Entry::default();

        if ctx.registration.state() == RegistrationState::Finished {
            let word = ctx.registration.word().to_string();
            ctx.output.fix(&word);
            ctx.registration.clear();
        }
    }

    fn input(&mut self, ctx: &mut InputContext, fixed: &str) {
        ctx.output.fix(fixed);
    }

    fn input_event(&mut self, ctx: &mut InputContext) {
        ctx.event_handled = false;
    }

    fn commit(&mut self, ctx: &mut InputContext, queue: &mut String) {
        ctx.output.fix(queue);
        queue.clear();
        ctx.entry = Entry::default();
    }
}

/// Reading (見出し語) editor (port of SKKComposingEditor).
#[derive(Debug, Clone, Default)]
pub struct ComposingEditor {
    composing: TextBuffer,
}

impl ComposingEditor {
    fn read_context(&mut self, ctx: &mut InputContext) {
        self.composing.clear();

        if ctx.entry.is_empty() {
            // Entered from direct input; may carry an undo entry
            self.composing.insert(&ctx.undo.entry().to_string());
        } else {
            // Entered back from conversion; restore the reading
            ctx.entry.set_okuri("", "");
            self.composing.insert(&ctx.entry.entry_string().to_string());
        }

        ctx.dynamic_completion = true;
    }

    fn write_context(&mut self, ctx: &mut InputContext) {
        ctx.output.set_mark();
        let str_ = format!("▽{}", self.composing.string());
        ctx.output.compose(&str_, self.composing.cursor_position());

        self.update(ctx);
    }

    fn input(&mut self, ctx: &mut InputContext, fixed: &str) {
        self.composing.insert(fixed);
        self.update(ctx);
    }

    fn input_event(&mut self, ctx: &mut InputContext, event: EditorEvent) {
        match event {
            EditorEvent::BackSpace => {
                if self.composing.is_empty() {
                    ctx.needs_setback = true;
                }
                self.composing.backspace();
            }
            EditorEvent::Delete => self.composing.delete(),
            EditorEvent::CursorLeft => self.composing.cursor_left(),
            EditorEvent::CursorRight => self.composing.cursor_right(),
            EditorEvent::CursorUp => self.composing.cursor_up(),
            EditorEvent::CursorDown => self.composing.cursor_down(),
        }

        self.update(ctx);
    }

    fn commit(&mut self, queue: &mut String) {
        *queue = self.composing.string();
    }

    pub fn set_entry(&mut self, ctx: &mut InputContext, entry: &str) {
        self.composing.clear();
        self.composing.insert(entry);
        self.update(ctx);
    }

    fn update(&mut self, ctx: &mut InputContext) {
        ctx.entry = Entry::from_entry(&self.composing.left_string());
    }
}

/// Okurigana editor (port of SKKOkuriEditor).
#[derive(Debug, Clone, Default)]
pub struct OkuriEditor {
    first: bool,
    prefix: String,
    okuri: String,
    input: String,
}

impl OkuriEditor {
    fn read_context(&mut self, _ctx: &mut InputContext) {
        self.first = true;
        self.prefix.clear();
        self.okuri.clear();
    }

    fn write_context(&mut self, ctx: &mut InputContext) {
        let str_ = format!("*{}", self.okuri);
        ctx.output.compose(&str_, 0);

        self.update(ctx);
    }

    /// Returns text to append to the reading (the KesSi case).
    #[must_use]
    fn input(
        &mut self,
        ctx: &mut InputContext,
        fixed: &str,
        input: &str,
        code: u8,
    ) -> Option<String> {
        self.input = input.to_string();

        if self.first {
            self.first = false;
            self.prefix.push(code.to_ascii_lowercase() as char);

            // KesSi: consonant typed while the previous one was pending
            if !fixed.is_empty() && !input.is_empty() {
                self.update(ctx);
                return Some(fixed.to_string());
            }
        }

        // ASCII fixed strings are not okurigana; detect by width
        if fixed.chars().count() != fixed.len() {
            self.okuri += fixed;
        }

        // OWsa: retarget the prefix while no okurigana is fixed yet
        if self.okuri.is_empty() {
            self.prefix.clear();
            if input.is_empty() {
                if code != 0 {
                    self.prefix.push(code.to_ascii_lowercase() as char);
                }
            } else {
                self.prefix.push(input.as_bytes()[0].to_ascii_lowercase() as char);
            }
        } else if self.prefix.is_empty() && code != 0 {
            self.prefix.push(code.to_ascii_lowercase() as char);
        }

        self.update(ctx);
        None
    }

    fn input_event(&mut self, ctx: &mut InputContext, event: EditorEvent) {
        if event == EditorEvent::BackSpace {
            if self.okuri.is_empty() {
                ctx.needs_setback = true;
            } else {
                let mut chars: Vec<char> = self.okuri.chars().collect();
                chars.pop();
                self.okuri = chars.into_iter().collect();
            }
        }

        self.update(ctx);
    }

    fn commit(&mut self) {
        self.prefix.clear();
        self.okuri.clear();
    }

    pub fn is_okuri_complete(&self) -> bool {
        !self.okuri.is_empty() && self.input.is_empty()
    }

    fn update(&mut self, ctx: &mut InputContext) {
        ctx.entry.set_okuri(&self.prefix, &self.okuri);
    }
}

/// Selected-candidate editor (port of SKKCandidateEditor).
#[derive(Debug, Clone, Default)]
pub struct CandidateEditor {
    entry: Entry,
    candidate: Candidate,
}

impl CandidateEditor {
    fn read_context(&mut self, ctx: &mut InputContext) {
        self.entry = ctx.entry.clone();
        ctx.annotation = true;
    }

    fn write_context(&mut self, ctx: &mut InputContext) {
        let mut str_ = self.candidate.variant().to_string();

        if self.entry.is_okuri_ari() {
            str_ += self.entry.okuri_string();
        }

        ctx.output.set_mark();
        ctx.output.convert(&format!("▼{str_}"));

        self.update(ctx);
    }

    fn commit(&mut self, backend: &mut Backend, queue: &mut String) {
        backend.register(&self.entry, &self.candidate);

        *queue = self.candidate.variant().to_string();
        self.candidate = Candidate::default();

        if self.entry.is_okuri_ari() {
            *queue += self.entry.okuri_string();
        }
    }

    pub fn set_candidate(&mut self, ctx: &mut InputContext, candidate: &Candidate) {
        self.candidate = candidate.clone();
        self.update(ctx);
    }

    fn update(&mut self, ctx: &mut InputContext) {
        ctx.entry = self.entry.clone();
        ctx.candidate = self.candidate.clone();
    }
}

/// Entry removal confirmation (port of SKKEntryRemoveEditor).
#[derive(Debug, Clone, Default)]
pub struct EntryRemoveEditor {
    entry: Entry,
    candidate: Candidate,
    prompt: String,
    input: String,
}

impl EntryRemoveEditor {
    fn read_context(&mut self, ctx: &mut InputContext) {
        self.entry = ctx.entry.clone();
        self.candidate = ctx.candidate.clone();
        self.input.clear();

        self.prompt = format!(
            "{} /{}/ を削除しますか？(yes/no) ",
            self.entry.entry_string(),
            self.candidate.to_string()
        );
    }

    fn write_context(&mut self, ctx: &mut InputContext) {
        ctx.output.clear();
        let str_ = format!("{}{}", self.prompt, self.input);
        ctx.output.compose(&str_, 0);

        ctx.entry = self.entry.clone();
    }

    fn input(&mut self, fixed: &str) {
        self.input += fixed;
    }

    fn input_event(&mut self, event: EditorEvent) {
        if event == EditorEvent::BackSpace && !self.input.is_empty() {
            self.input.pop();
        }
    }

    fn commit(&mut self, ctx: &mut InputContext, backend: &mut Backend, queue: &mut String) {
        if self.input != "yes" {
            ctx.needs_setback = true;
        } else {
            backend.remove(&self.entry, &self.candidate);
        }

        queue.clear();
    }
}

/// Word registration editor (port of SKKRegisterEditor).
#[derive(Debug, Clone, Default)]
pub struct RegisterEditor {
    entry: Entry,
    prompt: String,
    word: TextBuffer,
}

impl RegisterEditor {
    pub fn new(entry: &Entry) -> Self {
        Self {
            entry: entry.clone(),
            prompt: format!("[登録：{}]", entry.prompt_string()),
            word: TextBuffer::default(),
        }
    }

    fn read_context(&mut self, ctx: &mut InputContext) {
        ctx.entry = Entry::default();

        let word = ctx.registration.word().to_string();
        self.input(&word);
        ctx.registration.clear();
    }

    fn write_context(&mut self, ctx: &mut InputContext) {
        let str_ = format!("{}{}", self.prompt, self.word.string());
        ctx.output.compose(&str_, self.word.cursor_position());
    }

    fn input(&mut self, fixed: &str) {
        self.word.insert(fixed);
    }

    fn input_event(&mut self, event: EditorEvent) {
        match event {
            EditorEvent::BackSpace => self.word.backspace(),
            EditorEvent::Delete => self.word.delete(),
            EditorEvent::CursorLeft => self.word.cursor_left(),
            EditorEvent::CursorRight => self.word.cursor_right(),
            EditorEvent::CursorUp => self.word.cursor_up(),
            EditorEvent::CursorDown => self.word.cursor_down(),
        }
    }

    fn commit(&mut self, ctx: &mut InputContext, queue: &mut String) {
        self.word.insert(queue);
        *queue = self.word.string();

        ctx.entry = self.entry.clone();
    }
}

// ======================================================================
// Input engine (port of SKKInputEngine)
// ======================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorId {
    Bottom,
    Composing,
    Okuri,
    Candidate,
    EntryRemove,
}

/// The bottom of the editor stack: direct input at the top level,
/// word registration in nested levels.
#[derive(Debug, Clone)]
pub enum BottomEditor {
    Primary(PrimaryEditor),
    Register(RegisterEditor),
}

/// External dependencies passed into engine operations.
pub struct EngineDeps<'a> {
    pub backend: &'a mut Backend,
    pub converter: &'a RomanKanaConverter,
    pub config: &'a Config,
}

/// The editor stack coordinator (port of SKKInputEngine).
#[derive(Debug, Clone)]
pub struct InputEngine {
    mode: InputMode,
    mode_changed: bool,

    stack: Vec<EditorId>,
    input_queue: InputQueue,
    input_state: QueueState,
    word: String,

    bottom: BottomEditor,
    composing: ComposingEditor,
    okuri: OkuriEditor,
    candidate_editor: CandidateEditor,
    entry_remove: EntryRemoveEditor,
}

impl InputEngine {
    pub fn new(bottom: BottomEditor) -> Self {
        Self {
            mode: InputMode::Hirakana,
            mode_changed: false,
            stack: vec![EditorId::Bottom],
            input_queue: InputQueue::default(),
            input_state: QueueState::default(),
            word: String::new(),
            bottom,
            composing: ComposingEditor::default(),
            okuri: OkuriEditor::default(),
            candidate_editor: CandidateEditor::default(),
            entry_remove: EntryRemoveEditor::default(),
        }
    }

    pub fn input_mode(&self) -> InputMode {
        self.mode
    }

    /// True when the input mode changed since the last check
    /// (replaces the original's listener notifications).
    pub fn take_mode_changed(&mut self) -> bool {
        std::mem::take(&mut self.mode_changed)
    }

    pub fn select_input_mode(&mut self, ctx: &mut InputContext, mode: InputMode) {
        if self.mode != mode {
            self.mode_changed = true;
        }
        self.mode = mode;
        let state = self.input_queue.select_input_mode(mode);
        self.route(ctx, state);

        ctx.event_handled = true;
    }

    // ------------------------------------------------------------
    // State changes (port of SetState*)
    // ------------------------------------------------------------

    fn sync_begin(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.update_input_context(ctx, deps.config);
        self.initialize(ctx);
    }

    fn sync_end(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.editor_read_context(*self.stack.last().unwrap(), ctx);
        self.update_input_context(ctx, deps.config);
    }

    fn initialize(&mut self, ctx: &mut InputContext) {
        self.stack.clear();
        self.stack.push(EditorId::Bottom);

        ctx.dynamic_completion = false;
        ctx.annotation = false;

        if ctx.registration.state() == RegistrationState::Aborted {
            ctx.registration.clear();
            self.mode_changed = true; // refresh mode display
        }
    }

    pub fn set_state_primary(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.sync_begin(ctx, deps);
        self.sync_end(ctx, deps);
    }

    pub fn set_state_composing(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.sync_begin(ctx, deps);
        self.stack.push(EditorId::Composing);

        if !deps.config.delete_okuri_when_quit {
            let okuri = ctx.entry.okuri_string().to_string();
            ctx.entry.append_entry(&okuri);
        }

        self.sync_end(ctx, deps);
    }

    pub fn set_state_okuri(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.sync_begin(ctx, deps);
        self.stack.push(EditorId::Composing);
        self.stack.push(EditorId::Okuri);
        self.sync_end(ctx, deps);
    }

    pub fn set_state_select_candidate(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.sync_begin(ctx, deps);
        self.stack.push(EditorId::Candidate);
        self.sync_end(ctx, deps);
    }

    pub fn set_state_entry_remove(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.sync_begin(ctx, deps);
        self.stack.push(EditorId::EntryRemove);
        self.sync_end(ctx, deps);
    }

    pub fn set_state_registration(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.update_input_context(ctx, deps.config);
        ctx.registration.start();
    }

    // ------------------------------------------------------------
    // Input handling
    // ------------------------------------------------------------

    pub fn handle_char(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps, code: u8, direct: bool) {
        let state = self.input_queue.add_char(deps.converter, code, direct);
        self.route(ctx, state);
    }

    pub fn handle_backspace(&mut self, ctx: &mut InputContext) {
        match self.input_queue.remove_char() {
            Some(state) => self.route(ctx, state),
            None => self.invoke(ctx, EditorEvent::BackSpace),
        }
    }

    pub fn handle_delete(&mut self, ctx: &mut InputContext) {
        self.invoke(ctx, EditorEvent::Delete);
    }

    pub fn handle_cursor_left(&mut self, ctx: &mut InputContext) {
        self.invoke(ctx, EditorEvent::CursorLeft);
    }

    pub fn handle_cursor_right(&mut self, ctx: &mut InputContext) {
        self.invoke(ctx, EditorEvent::CursorRight);
    }

    pub fn handle_cursor_up(&mut self, ctx: &mut InputContext) {
        self.invoke(ctx, EditorEvent::CursorUp);
    }

    pub fn handle_cursor_down(&mut self, ctx: &mut InputContext) {
        self.invoke(ctx, EditorEvent::CursorDown);
    }

    pub fn handle_paste(&mut self, ctx: &mut InputContext, text: &str) {
        self.editor_input_ascii(*self.stack.last().unwrap(), ctx, text);
    }

    pub fn handle_enter(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.commit(ctx, deps);

        let candidate = Candidate::from_word(&self.word);
        let entry = ctx.entry.clone();
        self.study(deps, &entry, &candidate);

        if self.word.is_empty() {
            ctx.registration.abort();
        } else {
            let output = format!("{}{}", self.word, ctx.entry.okuri_string());
            ctx.registration.finish(&output);
        }

        ctx.event_handled = false;
    }

    pub fn handle_cancel(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        if !self.input_queue.is_empty() {
            self.terminate(ctx, deps);
            return;
        }

        ctx.registration.abort();
        ctx.event_handled = false;
    }

    pub fn commit(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.terminate(ctx, deps);

        self.word.clear();

        let mut word = std::mem::take(&mut self.word);
        for index in (0..self.stack.len()).rev() {
            self.editor_commit(self.stack[index], ctx, deps.backend, &mut word);
        }
        self.word = word;
    }

    pub fn reset(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.terminate(ctx, deps);
        ctx.event_handled = false;
    }

    pub fn toggle_kana(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.terminate(ctx, deps);

        let entry = ctx.entry.clone();
        self.study(deps, &entry, &Candidate::default());

        let str_ = entry.toggle_kana(self.mode);
        self.insert(ctx, &str_);
    }

    pub fn toggle_jisx0201_kana(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        self.terminate(ctx, deps);

        let entry = ctx.entry.clone();
        self.study(deps, &entry, &Candidate::default());

        let str_ = entry.toggle_jisx0201_kana(self.mode);
        self.insert(ctx, &str_);
    }

    /// Rebuild the composing output from the editor stack.
    pub fn update_input_context(&mut self, ctx: &mut InputContext, config: &Config) {
        ctx.output.clear();

        for index in 0..self.stack.len() {
            self.editor_write_context(self.stack[index], ctx);
        }

        // Pending romaji, or its shortest kana match (ex. "ky")
        if config.display_shortest_match_of_kana_conversions
            && !self.input_state.intermediate.is_empty()
        {
            let str_ = self.input_state.intermediate.clone();
            ctx.output.compose(&str_, 0);
        } else {
            let str_ = self.input_state.queue.clone();
            ctx.output.compose(&str_, 0);
        }
    }

    pub fn can_convert(&self, converter: &RomanKanaConverter, code: u8) -> bool {
        self.input_queue.can_convert(converter, code)
    }

    pub fn is_okuri_complete(&self) -> bool {
        self.okuri.is_okuri_complete()
    }

    // ------------------------------------------------------------
    // Buddy interfaces (completer / selector)
    // ------------------------------------------------------------

    /// Port of `SKKSelectorQueryEntry`.
    pub fn selector_query_entry(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) -> Entry {
        self.terminate(ctx, deps);

        let entry = ctx.entry.normalize(self.mode);
        ctx.entry = entry.clone();

        entry
    }

    /// Port of `SKKCompleterQueryString`.
    pub fn completer_query_string(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) -> String {
        self.selector_query_entry(ctx, deps).entry_string().to_string()
    }

    /// Port of `SKKSelectorUpdate`.
    pub fn set_candidate(&mut self, ctx: &mut InputContext, candidate: &Candidate) {
        self.candidate_editor.set_candidate(ctx, candidate);
    }

    /// Port of `SKKCompleterUpdate`.
    pub fn set_composing_entry(&mut self, ctx: &mut InputContext, entry: &str) {
        self.composing.set_entry(ctx, entry);
    }

    // ------------------------------------------------------------
    // Internals
    // ------------------------------------------------------------

    fn terminate(&mut self, ctx: &mut InputContext, deps: &mut EngineDeps) {
        let state = if deps.config.fix_intermediate_conversion {
            self.input_queue.terminate(deps.converter)
        } else if self.input_queue.is_empty() {
            None
        } else {
            Some(self.input_queue.clear())
        };

        if let Some(state) = state {
            self.route(ctx, state);
        }
    }

    fn invoke(&mut self, ctx: &mut InputContext, event: EditorEvent) {
        if !self.input_queue.is_empty() {
            self.input_queue.clear();
            self.input_state = QueueState::default();
            ctx.event_handled = false;
        } else {
            self.editor_input_event(*self.stack.last().unwrap(), ctx, event);
        }
    }

    fn study(&mut self, deps: &mut EngineDeps, entry: &Entry, candidate: &Candidate) {
        if entry.is_empty() {
            return;
        }
        if entry.is_okuri_ari() && (entry.okuri_string().is_empty() || candidate.is_empty()) {
            return;
        }

        deps.backend.register(entry, candidate);
    }

    fn insert(&mut self, ctx: &mut InputContext, str_: &str) {
        // Route to the bottom editor
        self.editor_input(EditorId::Bottom, ctx, str_, "", 0);
    }

    /// Port of `SKKInputQueueUpdate`: route conversion output to the
    /// top editor.
    fn route(&mut self, ctx: &mut InputContext, state: QueueState) {
        self.input_state = state.clone();
        let top = *self.stack.last().unwrap();

        if self.mode == InputMode::Ascii {
            self.editor_input_ascii(top, ctx, &state.fixed);
        } else {
            self.editor_input(top, ctx, &state.fixed, &state.queue, state.code);
        }
    }

    // ------------------------------------------------------------
    // Editor dispatch
    // ------------------------------------------------------------

    fn editor_read_context(&mut self, id: EditorId, ctx: &mut InputContext) {
        match id {
            EditorId::Bottom => match &mut self.bottom {
                BottomEditor::Primary(editor) => editor.read_context(ctx),
                BottomEditor::Register(editor) => editor.read_context(ctx),
            },
            EditorId::Composing => self.composing.read_context(ctx),
            EditorId::Okuri => self.okuri.read_context(ctx),
            EditorId::Candidate => self.candidate_editor.read_context(ctx),
            EditorId::EntryRemove => self.entry_remove.read_context(ctx),
        }
    }

    fn editor_write_context(&mut self, id: EditorId, ctx: &mut InputContext) {
        match id {
            EditorId::Bottom => match &mut self.bottom {
                BottomEditor::Primary(_) => {}
                BottomEditor::Register(editor) => editor.write_context(ctx),
            },
            EditorId::Composing => self.composing.write_context(ctx),
            EditorId::Okuri => self.okuri.write_context(ctx),
            EditorId::Candidate => self.candidate_editor.write_context(ctx),
            EditorId::EntryRemove => self.entry_remove.write_context(ctx),
        }
    }

    fn editor_input(&mut self, id: EditorId, ctx: &mut InputContext, fixed: &str, input: &str, code: u8) {
        match id {
            EditorId::Bottom => match &mut self.bottom {
                BottomEditor::Primary(editor) => editor.input(ctx, fixed),
                BottomEditor::Register(editor) => editor.input(fixed),
            },
            EditorId::Composing => self.composing.input(ctx, fixed),
            EditorId::Okuri => {
                if let Some(append) = self.okuri.input(ctx, fixed, input, code) {
                    // KesSi: the fixed part belongs to the reading
                    self.composing.input(ctx, &append);
                    // Restore okuri prompt state after composing update
                    self.okuri.update_entry(ctx);
                }
            }
            EditorId::EntryRemove => self.entry_remove.input(fixed),
            EditorId::Candidate => {}
        }
    }

    fn editor_input_ascii(&mut self, id: EditorId, ctx: &mut InputContext, ascii: &str) {
        match id {
            EditorId::Bottom => match &mut self.bottom {
                BottomEditor::Primary(editor) => {
                    // Ascii input passes through unhandled in primary mode
                    let _ = editor;
                    ctx.event_handled = false;
                }
                BottomEditor::Register(editor) => editor.input(ascii),
            },
            EditorId::Composing => self.composing.input(ctx, ascii),
            EditorId::EntryRemove => self.entry_remove.input(ascii),
            EditorId::Okuri | EditorId::Candidate => {}
        }
    }

    fn editor_input_event(&mut self, id: EditorId, ctx: &mut InputContext, event: EditorEvent) {
        match id {
            EditorId::Bottom => match &mut self.bottom {
                BottomEditor::Primary(editor) => editor.input_event(ctx),
                BottomEditor::Register(editor) => editor.input_event(event),
            },
            EditorId::Composing => self.composing.input_event(ctx, event),
            EditorId::Okuri => self.okuri.input_event(ctx, event),
            EditorId::EntryRemove => self.entry_remove.input_event(event),
            EditorId::Candidate => {}
        }
    }

    fn editor_commit(&mut self, id: EditorId, ctx: &mut InputContext, backend: &mut Backend, queue: &mut String) {
        match id {
            EditorId::Bottom => match &mut self.bottom {
                BottomEditor::Primary(editor) => editor.commit(ctx, queue),
                BottomEditor::Register(editor) => editor.commit(ctx, queue),
            },
            EditorId::Composing => self.composing.commit(queue),
            EditorId::Okuri => self.okuri.commit(),
            EditorId::Candidate => self.candidate_editor.commit(backend, queue),
            EditorId::EntryRemove => self.entry_remove.commit(ctx, backend, queue),
        }
    }
}

impl OkuriEditor {
    fn update_entry(&mut self, ctx: &mut InputContext) {
        self.update(ctx);
    }
}
