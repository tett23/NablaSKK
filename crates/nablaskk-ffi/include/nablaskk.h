/* C ABI for nablaskk-core (Rust reimplementation of the AquaSKK engine).
 *
 * All strings are NUL-terminated UTF-8. Strings returned by skk_*
 * functions are owned by the caller: release them with skk_string_free.
 *
 * License: GPL-2.0-or-later.
 */

#ifndef NABLASKK_H
#define NABLASKK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SkkSession SkkSession;

/* Dictionary types (DictionarySet numbering of the original AquaSKK). */
enum {
    SKK_DICT_COMMON = 0,      /* SKK-JISYO, EUC-JP / UTF-8 auto-detected */
    SKK_DICT_AUTO_UPDATE = 1, /* "host url path", downloaded over HTTP */
    SKK_DICT_PROXY = 2,       /* "host:port" of a running skkserv */
    SKK_DICT_GADGET = 4,      /* today/now/=expr dynamic candidates */
    SKK_DICT_COMMON_UTF8 = 5  /* SKK-JISYO, UTF-8 */
};

/* Modifier bits for skk_session_handle (matches keymap.conf). */
enum {
    SKK_MOD_SHIFT = 1 << 1,
    SKK_MOD_CTRL = 1 << 2,
    SKK_MOD_ALT = 1 << 3,
    SKK_MOD_META = 1 << 4
};

/* Input modes returned by skk_session_input_mode. */
enum {
    SKK_MODE_HIRAKANA = 0,
    SKK_MODE_KATAKANA = 1,
    SKK_MODE_JISX0201KANA = 2,
    SKK_MODE_ASCII = 3,
    SKK_MODE_JISX0208LATIN = 4
};

/* Create a session with the default kana rules and keymap.
 * user_dictionary_path may be NULL (defaults to ~/.rust-skk-jisyo). */
SkkSession *skk_session_new(const char *user_dictionary_path);
void skk_session_free(SkkSession *session);

/* Add a system dictionary. dictionary_type: 0=SKK-JISYO (encoding
 * auto-detected), 1=auto-update, 2=skkserv (EUC-JP), 4=gadget,
 * 5=SKK-JISYO (UTF-8), 6=skkserv (UTF-8). Returns 0 on success. */
int32_t skk_session_add_dictionary(SkkSession *session, int32_t dictionary_type,
                                   const char *location);

/* Restore the built-in romaji-kana rules, dropping applied patches. */
void skk_session_reset_kana_rules(SkkSession *session);
/* Merge kana-rule text (UTF-8, kana-rule.conf format) into the rules. */
void skk_session_patch_kana_rules(SkkSession *session, const char *rules);

/* Restore the built-in keymap, dropping applied overrides. */
void skk_session_reset_keymap(SkkSession *session);
/* Rebind keys from keymap.conf-format text; each line replaces every key
 * currently bound to its symbol. */
void skk_session_override_keymap(SkkSession *session, const char *text);

/* Remove every system dictionary (the user dictionary is kept). */
void skk_session_clear_dictionaries(SkkSession *session);

/* Feed one key event. Returns 1 when consumed by the IME. */
int32_t skk_session_handle(SkkSession *session, uint8_t charcode,
                           uint8_t keycode, uint32_t mods);

/* Set the text a paste event (Ctrl-Y / Cmd-V) will insert; call with the
 * system pasteboard contents before feeding the paste key. */
void skk_session_set_clipboard(SkkSession *session, const char *text);

/* Text committed since the last call (caller frees). */
char *skk_session_take_fixed(SkkSession *session);

/* Current marked (composing) text (caller frees). */
char *skk_session_composing(const SkkSession *session);

/* Cursor within the composing text: character offset from its end
 * (0 = end, -1 = before the last character). */
int32_t skk_session_composing_cursor(const SkkSession *session);

int32_t skk_session_input_mode(const SkkSession *session);

void skk_session_commit(SkkSession *session);
void skk_session_clear(SkkSession *session);
void skk_session_save(SkkSession *session);
/* Re-read the user dictionary if another program changed the file. */
int32_t skk_session_reload_user_dictionary(SkkSession *session);

/* Candidate window state (valid until the next handled event). */
int32_t skk_session_candidates_visible(const SkkSession *session);
int32_t skk_session_candidate_count(const SkkSession *session);
char *skk_session_candidate(const SkkSession *session, int32_t index);
int32_t skk_session_candidate_cursor(const SkkSession *session);
/* Returns (page << 16) | page_count, page is 1-based. */
int32_t skk_session_candidate_page(const SkkSession *session);

/* Engine options for skk_session_set_option. */
enum {
    SKK_OPTION_SUPPRESS_NEWLINE_ON_COMMIT = 0,
    SKK_OPTION_INLINE_BACKSPACE_IMPLIES_COMMIT = 1,
    SKK_OPTION_DELETE_OKURI_WHEN_QUIT = 2,
    SKK_OPTION_HANDLE_RECURSIVE_ENTRY_AS_OKURI = 3,
    SKK_OPTION_FIX_INTERMEDIATE_CONVERSION = 4,
    SKK_OPTION_DISPLAY_SHORTEST_MATCH = 5,
    SKK_OPTION_USE_NUMERIC_CONVERSION = 6,
    SKK_OPTION_MAX_INLINE_CANDIDATES = 7
};

/* Set an engine option. Returns 0 on success. */
int32_t skk_session_set_option(SkkSession *session, int32_t option,
                               int32_t value);

void skk_string_free(char *str);

#ifdef __cplusplus
}
#endif

#endif /* NABLASKK_H */
