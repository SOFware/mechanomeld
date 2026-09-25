# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [0.1.2] - Unreleased

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
