#!/usr/bin/env node

import { strict as assert } from "node:assert";
import { QwenByteBpeTokenizer } from "../../apps/jbotci-web/assets/f2llm-webgpu-runtime.js";

const tokenizer = new QwenByteBpeTokenizer({
  eosId: 999,
  vocab: {
    h: 1,
    e: 2,
    l: 3,
    o: 4,
    "he": 5,
    "hel": 6,
    "hell": 7,
    "hello": 8,
    "Ġ": 9,
    w: 10,
    r: 11,
    d: 12,
    "wo": 13,
    "wor": 14,
    "worl": 15,
    "world": 16,
    "Ã": 17,
    "©": 18,
    "Ã©": 19,
    "!": 20,
    ".": 21,
    "Ċ": 22,
  },
  merges: [
    "h e",
    "he l",
    "hel l",
    "hell o",
    "w o",
    "wo r",
    "wor l",
    "worl d",
    "Ã ©",
  ],
});

assert.deepEqual(tokenizer.encode("hello", 8), [8, 999]);
assert.deepEqual(tokenizer.encode("hello world", 8), [8, 9, 16, 999]);
assert.deepEqual(tokenizer.encode("é", 8), [19, 999]);
assert.deepEqual(tokenizer.encode("hello world!", 3), [8, 9, 999]);
assert.deepEqual(tokenizer.encode("hello\n", 8), [8, 22, 999]);

console.log("f2llm tokenizer tests passed");
