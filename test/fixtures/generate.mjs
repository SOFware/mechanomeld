// Writes the .automerge fixtures read by the Ruby tests. Run with `mise run fixtures`.
import * as A from "@automerge/automerge";
import { Repo } from "@automerge/automerge-repo";
import { NodeFSStorageAdapter } from "@automerge/automerge-repo-storage-nodefs";
import { rmSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// heads.json records Automerge.getHeads for each fixture, by name or repo document id,
// so the Ruby tests can check Document#heads against it.
const heads = {};
const save = (name, doc) => {
  writeFileSync(new URL(`${name}.automerge`, import.meta.url), A.save(doc));
  heads[name] = A.getHeads(doc);
};

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

// A NodeFSStorageAdapter directory, as automerge-repo writes it, for Document.load_repo.
// Document ids are fixed so test/test_document_load_repo.rb can name them.
const repoDir = fileURLToPath(new URL("repo", import.meta.url));
rmSync(repoDir, { recursive: true, force: true });
const storage = new Repo({ storage: new NodeFSStorageAdapter(repoDir) }).storageSubsystem;

const snapshotOnly = A.from({ count: new A.Counter(17), title: "snapshot only" });
await storage.saveDoc("2be82g1cnxj1o64J7cFA5wPeyHms", snapshotOnly);

// automerge-repo compacts every save into a new snapshot while the snapshot is under
// 1024 bytes, so this document starts with enough items to get incremental chunks.
const incrementalId = "3bTvdjpAxqE36opq4dAe5sD6ps6o";
const items = Array.from({ length: 150 }, (_, i) => `item-${i}`);
let incremental = A.from({ count: new A.Counter(1), items });
await storage.saveDoc(incrementalId, incremental);
incremental = A.change(incremental, (doc) => {
  doc.count.increment(4);
  doc.items.push("first");
});
await storage.saveDoc(incrementalId, incremental);
incremental = A.change(incremental, (doc) => {
  doc.count.increment(5);
  doc.items.push("second");
});
await storage.saveDoc(incrementalId, incremental);
await storage.saveSyncState(incrementalId, "peer-storage-id", A.initSyncState());
heads[incrementalId] = A.getHeads(incremental);

writeFileSync(new URL("heads.json", import.meta.url), JSON.stringify(heads, null, 2) + "\n");
