# SSE Event Replay with Last-Event-ID

**Labels:** `enhancement`, `backend`, `frontend`, `PRD-10`
**Difficulty:** Hard
**PRD:** [PRD-10: Durable Event Log](../../prds/PRD-10-durable-event-log.md) — Phase 5
**Depends on:** #009

## Summary

Update the SSE `/events` endpoint to support the standard `Last-Event-ID` header for replay. When a client reconnects after a brief disconnect, it receives all events it missed instead of starting from scratch. Also update the frontend SSE client to send `Last-Event-ID` on reconnect.

## Why this is hard

- The SSE endpoint currently streams from `tokio::broadcast` — must be rewired to stream from the event log
- Need to handle the transition from historical replay to live streaming without gaps or duplicates
- The frontend `subscribeEvents()` function in `ui/src/lib/api.ts` needs reconnect logic with `Last-Event-ID`
- Must handle the case where requested sequence is older than retained events (event log was trimmed)
- Race condition: events appended between reading history and subscribing to live stream

## What to do

### Backend

1. Update the SSE handler in `crates/api/src/lib.rs`:
   - Extract `Last-Event-ID` from request headers (or `?since=` query param)
   - If provided: read historical events from `EventLog`, then switch to live streaming
   - Each SSE frame must include `id: {sequence}` field
   - If the requested sequence is too old (trimmed), start from the earliest available and indicate gap

2. Implementation approach:
   ```rust
   async fn sse_handler(
       headers: HeaderMap,
       event_log: Arc<dyn EventLog>,
       broadcast_rx: broadcast::Receiver<SystemEvent>,
   ) -> Sse<impl Stream<Item = Event>> {
       let last_id = headers.get("Last-Event-ID")
           .and_then(|v| v.to_str().ok())
           .and_then(|s| s.parse::<u64>().ok());

       let stream = async_stream::stream! {
           // Phase 1: Replay historical events
           if let Some(since) = last_id {
               let events = event_log.read_from(org_id, since + 1, 1000).await?;
               for stored in events {
                   yield Event::default()
                       .id(stored.sequence.to_string())
                       .data(serialize(&stored.event));
               }
           }

           // Phase 2: Switch to live stream
           loop {
               match broadcast_rx.recv().await {
                   Ok(event) => {
                       let seq = event_log.latest_sequence().await;
                       yield Event::default()
                           .id(seq.to_string())
                           .data(serialize(&event));
                   }
                   Err(_) => break,
               }
           }
       };

       Sse::new(stream)
   }
   ```

### Frontend

3. Update `subscribeEvents()` in `ui/src/lib/api.ts`:
   - Track the last received event ID
   - On `EventSource` error/close, reconnect with `Last-Event-ID` header
   - `EventSource` natively supports this via the `lastEventId` property

   ```typescript
   export function subscribeEvents(callback: (event: SpanEvent) => void) {
       let lastId: string | undefined;

       function connect() {
           const url = lastId
               ? `${API_BASE}/events?since=${lastId}`
               : `${API_BASE}/events`;
           const es = new EventSource(url);

           es.onmessage = (e) => {
               lastId = e.lastEventId;
               const event = JSON.parse(e.data);
               callback(event);
           };

           es.onerror = () => {
               es.close();
               setTimeout(connect, 2000);
           };
       }

       connect();
       return () => { /* cleanup */ };
   }
   ```

   Note: Standard `EventSource` sends `Last-Event-ID` automatically on reconnect if the server included `id:` fields. But if using a custom reconnect (as above), you may need to pass it as a query param since `EventSource` doesn't support custom headers.

## Files to modify

- `crates/api/src/lib.rs` — update SSE handler
- `crates/api/src/events.rs` — add `latest_sequence()` to `EventLog` trait if needed
- `ui/src/lib/api.ts` — update `subscribeEvents()` with reconnect + replay

## Acceptance criteria

- [ ] SSE frames include `id: {sequence}` field
- [ ] Reconnecting with `Last-Event-ID` replays missed events
- [ ] No duplicate events during replay-to-live transition
- [ ] Frontend auto-reconnects and replays on connection drop
- [ ] If events were trimmed, server handles gracefully (starts from earliest available)
- [ ] `cargo check -p api` and `npm run build` succeed
- [ ] Manual test: create spans, kill SSE connection, reconnect, verify missed spans arrive
