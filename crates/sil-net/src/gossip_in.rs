//! Messages gossip entrants : validés **avant** relais.

use crate::{NetEvent, Topic, Verdict, driver::Driver};
use libp2p::{
    PeerId,
    gossipsub::{self, MessageAcceptance, MessageId},
};

impl Driver {
    pub(crate) fn on_gossip(&mut self, from: PeerId, id: &MessageId, message: gossipsub::Message) {
        let topic = Topic::parse(message.topic.as_str());
        let verdict = match topic {
            Some(topic) => self.judge(&from, topic, &message.data),
            None => Verdict::Reject,
        };
        let acceptance = match verdict {
            Verdict::Accept => MessageAcceptance::Accept,
            Verdict::Reject => MessageAcceptance::Reject,
            Verdict::Ignore => MessageAcceptance::Ignore,
        };
        let _ = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .report_message_validation_result(id, &from, acceptance);
        if let (Verdict::Accept, Some(topic)) = (verdict, topic) {
            self.emit(NetEvent::Gossip {
                from: message.source.unwrap_or(from),
                topic,
                data: message.data,
            });
        }
    }

    fn judge(&mut self, from: &PeerId, topic: Topic, data: &[u8]) -> Verdict {
        let max = match topic {
            Topic::Presence => self.limits.max_presence_bytes,
            _ => self.limits.max_event_message_bytes,
        };
        if data.len() > max {
            return Verdict::Reject;
        }
        if !self.limiters.allow_gossip(from, self.clock.now_ms()) {
            return Verdict::Ignore;
        }
        self.providers.validator.validate(topic, data)
    }
}
