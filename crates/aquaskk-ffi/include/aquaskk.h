/* C ABI for aquaskk-core (Rust reimplementation of the AquaSKK engine).
 *
 * All strings are NUL-terminated UTF-8. Strings returned by skk_*
 * functions are owned by the caller: release them with skk_string_free.
 *
 * License: GPL-2.0-or-later.
 */

#ifndef AQUASKK_H
#define AQUASKK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SkkSession SkkSession;

/* Dictionary types (DictionarySet numbering of the original AquaSKK). */
enum {
    SKK_DICT_COMMON = 0,      /* SKK-JISYO, EUC-JP */
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

/* Add a system dictionary. Returns 0 on success. */
int32_t skk_session_add_dictionary(SkkSession *session, int32_t dictionary_type,
                                   const char *location);

/* Feed one key event. Returns 1 when consumed by the IME. */
int32_t skk_session_handle(SkkSession *session, uint8_t charcode,
                           uint8_t keycode, uint32_t mods);

/* Text committed since the last call (caller frees). */
char *skk_session_take_fixed(SkkSession *session);

/* Current marked (composing) text (caller frees). */
char *skk_session_composing(const SkkSession *session);

int32_t skk_session_input_mode(const SkkSession *session);

void skk_session_commit(SkkSession *session);
void skk_session_clear(SkkSession *session);
void skk_session_save(SkkSession *session);

void skk_string_free(char *str);

#ifdef __cplusplus
}
#endif

#endif /* AQUASKK_H */
