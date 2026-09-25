# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [0.1.1] - 2026-09-25

### Added

- Document#load_incremental applies saved Automerge bytes into an existing document (d60a3eb)
- Document.load_repo reads a document from automerge-repo's NodeFSStorageAdapter directory (d60a3eb)
- Document#heads returns the document's change hashes as hex, matching JavaScript's Automerge.getHeads (d60a3eb)
- Document#includes_heads?(heads) reports whether a document is at or past a set of heads (d60a3eb)
- Mechanomeld::SyncState, the per-peer sync state, with encode and decode for persistence (eea3a51)
- Document#generate_sync_message and #receive_sync_message, Automerge's per-document sync protocol (eea3a51)

### Changed

- releases also publish an x86_64-linux-musl platform gem (159dd59)

## [0.1.0] - 2026-09-17

### Added

- Initial release
- Mechanomeld::Counter, Timestamp, Uint, Bytes, and Text value classes (6df6951)
- Mechanomeld::Document.load reads documents saved by JavaScript Automerge (c7d31d8)
- Document#get, #[], and #to_h read values by path (c7d31d8)
- Document#keys and #length for maps, lists, and text (1d80c0b)
- Mechanomeld::Document.new(actor_id:) creates an empty document (bccb69c)
- Document#put, #[]=, #delete, and #save (bccb69c)
- Document#commit(message:, timestamp:) and #rollback (a8ef491)
- Document#change commits a block and rolls back if it raises (a8ef491)
- Mechanomeld::Document.from builds a committed document from a Hash (a8ef491)
