// NablaSKK: tests of the built wasm under Deno (ADR 0023). Replays the
// same testdata/scenarios.txt as the native test, so both builds must
// give the same states for the same keys.
//
//   deno test --allow-read crates/nablaskk-wasm/deno/wasm_test.ts
//
// NABLASKK_WASM overrides the module path (default: the release build).
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

const root = new URL("../../../", import.meta.url);
const wasmPath = Deno.env.get("NABLASKK_WASM") ??
  new URL("target/wasm32-unknown-unknown/release/nablaskk_wasm.wasm", root).pathname;
const dictionary = await Deno.readFile(new URL("data/SKK-JISYO.S", root));
const scenarios = await Deno.readTextFile(new URL("crates/nablaskk-wasm/testdata/scenarios.txt", root));

const module = new WebAssembly.Module(await Deno.readFile(wasmPath));

interface Exports {
  memory: WebAssembly.Memory;
  nablaskk_alloc(len: number): number;
  nablaskk_free(ptr: number, len: number): void;
  nablaskk_session_new(kp: number, kl: number, rp: number, rl: number, dp: number, dl: number): number;
  nablaskk_session_free(session: number): void;
  nablaskk_handle(session: number, charcode: number, keycode: number, mods: number): number;
  nablaskk_take_state(session: number): number;
  nablaskk_set_user_rules(session: number, ptr: number, len: number): number;
}

const SHIFT = 2;
const CTRL = 4;

// The page's use of the module: inputs through nablaskk_alloc, outputs
// as length-prefixed buffers released with nablaskk_free.
class Demo {
  #x: Exports;
  #session: number;

  constructor(keymap = "", rules = "", dict: Uint8Array = dictionary) {
    this.#x = new WebAssembly.Instance(module, {}).exports as unknown as Exports;
    const enc = new TextEncoder();
    const [k, r, d] = [enc.encode(keymap), enc.encode(rules), dict].map((b) => this.#put(b));
    this.#session = this.#x.nablaskk_session_new(k.ptr, k.len, r.ptr, r.len, d.ptr, d.len) >>> 0;
    for (const b of [k, r, d]) this.#x.nablaskk_free(b.ptr, b.len);
  }

  #put(bytes: Uint8Array) {
    const ptr = this.#x.nablaskk_alloc(bytes.length) >>> 0;
    new Uint8Array(this.#x.memory.buffer, ptr, bytes.length).set(bytes);
    return { ptr, len: bytes.length };
  }

  #take(ptr: number): string {
    ptr >>>= 0;
    const len = new DataView(this.#x.memory.buffer).getUint32(ptr, true);
    const text = new TextDecoder().decode(new Uint8Array(this.#x.memory.buffer, ptr + 4, len));
    this.#x.nablaskk_free(ptr, 4 + len);
    return text;
  }

  handle(charcode: number, keycode: number, mods: number): boolean {
    return this.#x.nablaskk_handle(this.#session, charcode, keycode, mods) === 1;
  }

  type(text: string) {
    for (const c of text) this.handle(c.charCodeAt(0), 0xff, /[A-Z]/.test(c) ? SHIFT : 0);
  }

  state(): string {
    return this.#take(this.#x.nablaskk_take_state(this.#session));
  }

  setUserRules(text: string): string {
    const b = this.#put(new TextEncoder().encode(text));
    const result = this.#take(this.#x.nablaskk_set_user_rules(this.#session, b.ptr, b.len));
    this.#x.nablaskk_free(b.ptr, b.len);
    return result;
  }

  free() {
    this.#x.nablaskk_session_free(this.#session);
  }
}

// Same notation as tests/common/mod.rs
const special: Record<string, [number, number, number]> = {
  SPC: [0x20, 49, 0],
  RET: [0x0d, 36, 0],
  TAB: [0x09, 48, 0],
  BS: [0x08, 51, 0],
  ESC: [0x1b, 53, 0],
  DEL: [0x7f, 117, 0],
  LEFT: [0, 123, 0],
  RIGHT: [0, 124, 0],
  DOWN: [0, 125, 0],
  UP: [0, 126, 0],
  lt: [0x3c, 0xff, SHIFT],
};

function parseKeys(keys: string): [number, number, number][] {
  const result: [number, number, number][] = [];
  for (const [, name, char] of keys.matchAll(/<([^>]+)>|(.)/g)) {
    if (name !== undefined) {
      const ctrl = /^C-(.)$/.exec(name);
      const key = ctrl ? [ctrl[1].charCodeAt(0), 0xff, CTRL] as [number, number, number] : special[name];
      if (!key) throw new Error(`unknown key <${name}>`);
      result.push(key);
    } else {
      const shifted = /[A-Z~!@#$%^&*()_+{}|:"<>?]/.test(char);
      result.push([char.charCodeAt(0), 0xff, shifted ? SHIFT : 0]);
    }
  }
  return result;
}

function assertEquals(actual: unknown, expected: unknown, message = "") {
  if (actual !== expected) {
    throw new Error(`${message}\n  expected: ${expected}\n    actual: ${actual}`);
  }
}

function field(json: string, name: string): string {
  return JSON.parse(json)[name];
}

Deno.test("imports nothing", () => {
  assertEquals(JSON.stringify(WebAssembly.Module.imports(module)), "[]");
});

Deno.test("scenarios match the native build", () => {
  let demo = new Demo();
  const failures: string[] = [];
  let count = 0;
  scenarios.split("\n").forEach((line, index) => {
    if (line === "" || line.startsWith("#")) return;
    const [command, input, expected] = line.split("\t");
    let actual: string;
    if (command === "new") {
      demo.free();
      demo = new Demo();
      return;
    } else if (command === "keys") {
      for (const [c, k, m] of parseKeys(input)) demo.handle(c, k, m);
      actual = demo.state();
    } else if (command === "rules") {
      actual = demo.setUserRules(input.replaceAll("\\n", "\n"));
    } else {
      throw new Error(`line ${index + 1}: malformed: ${line}`);
    }
    count++;
    if (actual !== expected) failures.push(`line ${index + 1}: ${input}\n  expected: ${expected}\n    actual: ${actual}`);
  });
  demo.free();
  if (count === 0 || failures.length > 0) {
    throw new Error(`${failures.length} / ${count} failed:\n${failures.join("\n")}`);
  }
});

Deno.test("kanji converts", () => {
  const demo = new Demo();
  demo.type("Kanji");
  demo.handle(0x20, 49, 0);
  assertEquals(field(demo.state(), "composing"), "▼漢字");
  demo.free();
});

Deno.test("okuri converts", () => {
  const demo = new Demo();
  demo.type("KaKu");
  assertEquals(field(demo.state(), "composing"), "▼書く");
  demo.free();
});

Deno.test("user rule fj types 、", () => {
  const demo = new Demo();
  const result = JSON.parse(demo.setUserRules("fj,、,、,､"));
  assertEquals(result[0].applied, true);
  demo.type("fj");
  assertEquals(field(demo.state(), "fixed"), "、");
  demo.free();
});

Deno.test("conflicting rule ka. is rejected with a reason", () => {
  const demo = new Demo();
  const result = JSON.parse(demo.setUserRules("ka.,…,…,…"));
  assertEquals(result[0].applied, false);
  assertEquals(result[0].reason, "「ka → か」がすぐに確定しなくなるため使えません");
  demo.type("ka");
  assertEquals(field(demo.state(), "fixed"), "か");
  demo.free();
});
