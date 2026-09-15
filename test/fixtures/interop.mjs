// Loads the document written by test/interop/write_fixtures.rb and checks JS sees the right values.
import * as A from "@automerge/automerge";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const doc = A.load(readFileSync(new URL("../interop/out/ruby.automerge", import.meta.url)));

assert.ok(doc.string instanceof A.ImmutableString, "Ruby String is an ImmutableString");
assert.equal(String(doc.string), "plain");
assert.equal(doc.text, "héllo 😀");
assert.equal(doc.int, -3);
assert.equal(doc.float, 2.5);
assert.equal(doc.bool, true);
assert.equal(doc.nothing, null);
assert.ok(doc.counter instanceof A.Counter, "Counter");
assert.equal(doc.counter.value, 5);
assert.ok(doc.timestamp instanceof Date, "Timestamp is a Date");
assert.equal(doc.timestamp.getTime(), 1700000000123);
assert.equal(doc.uint, 7n);
assert.deepEqual(Array.from(doc.bytes), [0, 1, 255]);
assert.equal(doc.nested.list[0], 1);
assert.equal(String(doc.nested.list[1]), "two");
assert.equal(doc.nested.list[2].three, 3);

console.log("interop ok");
