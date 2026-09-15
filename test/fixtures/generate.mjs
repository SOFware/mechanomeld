// Writes the .automerge fixtures read by the Ruby tests. Run with `mise run fixtures`.
import * as A from "@automerge/automerge";
import { writeFileSync } from "node:fs";

const save = (name, doc) =>
  writeFileSync(new URL(`${name}.automerge`, import.meta.url), A.save(doc));

const types = A.change(A.init(), (doc) => {
  doc.text = "héllo 😀";
  doc.string = new A.ImmutableString("fixed");
  doc.int = 42;
  doc.negative = new A.Int(-3);
  doc.float = 1.5;
  doc.whole_float = new A.Float64(2.0);
  doc.bool_true = true;
  doc.bool_false = false;
  doc.nothing = null;
  doc.counter = new A.Counter(5);
  doc.timestamp = new Date(1700000000123);
  doc.uint = new A.Uint(7);
  doc.bytes = new Uint8Array([0, 1, 255]);
  doc.nested = { list: [1, "two", { three: 3 }], empty_map: {}, empty_list: [] };
});
save("types", types);

// Two actors set the same key concurrently; actor "bbbb" wins.
let a = A.from({ k: new A.ImmutableString("base") }, { actor: "aaaa" });
let b = A.clone(a, { actor: "bbbb" });
a = A.change(a, (doc) => {
  doc.k = new A.ImmutableString("from-a");
});
b = A.change(b, (doc) => {
  doc.k = new A.ImmutableString("from-b");
});
save("conflict", A.merge(a, b));
