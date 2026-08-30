# Streaming

Streaming indicators provide O(1) per-bar incremental updates with checkpoint and restore support. Use the streaming API when processing live market data or large datasets where you only need the latest value without recomputing the full history.

See `docs/architecture/dataflow.md` for batch vs streaming data paths and the API reference for `StreamingIndicator` traits.
