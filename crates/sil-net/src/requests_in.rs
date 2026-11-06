//! Requêtes-réponses : on répond aux pairs (avec limite de débit) et on termine les nôtres.

use crate::{
    Bytes, ContentRequest, ContentResponse, LedgerRequest, LedgerResponse, NetError,
    driver::{Driver, complete},
    limits::CHUNK_RESPONSE_MAX,
};
use libp2p::{
    PeerId,
    request_response::{Event, Message},
};

impl Driver {
    pub(crate) fn on_ledger_event(&mut self, event: Event<LedgerRequest, LedgerResponse>) {
        match event {
            Event::Message {
                peer,
                message:
                    Message::Request {
                        request, channel, ..
                    },
                ..
            } => {
                let response = self.answer_ledger(&peer, &request);
                let _ = self
                    .swarm
                    .behaviour_mut()
                    .ledger
                    .send_response(channel, response);
            }
            Event::Message {
                message:
                    Message::Response {
                        request_id,
                        response,
                    },
                ..
            } => complete(&mut self.pending_ledger, request_id, Ok(response)),
            Event::OutboundFailure {
                request_id, error, ..
            } => complete(
                &mut self.pending_ledger,
                request_id,
                Err(NetError::Request(error.to_string())),
            ),
            _ => {}
        }
    }

    pub(crate) fn on_content_event(&mut self, event: Event<ContentRequest, ContentResponse>) {
        match event {
            Event::Message {
                peer,
                message:
                    Message::Request {
                        request, channel, ..
                    },
                ..
            } => {
                let response = self.answer_content(&peer, &request);
                let _ = self
                    .swarm
                    .behaviour_mut()
                    .content
                    .send_response(channel, response);
            }
            Event::Message {
                message:
                    Message::Response {
                        request_id,
                        response,
                    },
                ..
            } => complete(&mut self.pending_content, request_id, Ok(response)),
            Event::OutboundFailure {
                request_id, error, ..
            } => complete(
                &mut self.pending_content,
                request_id,
                Err(NetError::Request(error.to_string())),
            ),
            _ => {}
        }
    }

    fn answer_ledger(&mut self, peer: &PeerId, request: &LedgerRequest) -> LedgerResponse {
        if !self.limiters.allow_ledger(peer, self.clock.now_ms()) {
            return LedgerResponse::Busy;
        }
        let (events, max) = match request {
            LedgerRequest::Author {
                job,
                author,
                after_sequence,
                max,
            } => {
                let max = usize::from(*max).min(self.limits.max_ledger_events);
                (
                    self.providers
                        .ledger
                        .events_after(job, author, *after_sequence, max),
                    max,
                )
            }
            LedgerRequest::Cursor { job, cursor, max } => {
                let max = usize::from(*max).min(self.limits.max_ledger_events);
                (self.providers.ledger.events_since(job, *cursor, max), max)
            }
        };
        LedgerResponse::Events(
            events
                .into_iter()
                .filter(|event| event.len() <= self.limits.max_event_message_bytes)
                .take(max)
                .map(Bytes)
                .collect(),
        )
    }

    fn answer_content(&mut self, peer: &PeerId, request: &ContentRequest) -> ContentResponse {
        if !self.limiters.allow_content(peer, self.clock.now_ms()) {
            return ContentResponse::Busy;
        }
        match request {
            ContentRequest::Manifest { id } => self
                .providers
                .content
                .manifest(id)
                .map_or(ContentResponse::NotFound, |m| {
                    ContentResponse::Manifest(Bytes(m))
                }),
            ContentRequest::Chunk { id, index } => self
                .providers
                .content
                .chunk(id, *index)
                .filter(|chunk| (chunk.len() as u64) < CHUNK_RESPONSE_MAX)
                .map_or(ContentResponse::NotFound, |c| {
                    ContentResponse::Chunk(Bytes(c))
                }),
        }
    }
}
