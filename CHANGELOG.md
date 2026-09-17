# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

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
