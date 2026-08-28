//! Event data, held separately per game source.
//!
//! The UWH Portal serves a list of events the operator picks from. A custom
//! site serves no list at all — its single event is named inside the URL the
//! operator types, and refbox manufactures an entry for it so that teams and
//! schedule replies have somewhere to land and the court picker has a court
//! list to read.
//!
//! Both used to live in one map. That map backed the portal's event picker, so
//! switching from a custom site to the portal left the manufactured entry
//! sitting in the portal's list, offered as though the portal had served it —
//! and selecting it asked the portal for an event only the operator's own
//! server has. Holding the two apart, and naming the source on every read, is
//! what makes that unrepresentable rather than merely filtered out.
//!
//! Pure by design — no app state and no I/O — so every rule here is
//! unit-testable, which matters because the `update()` loop that uses it is not.

use crate::config::GameSource;
use std::collections::BTreeMap;
use uwh_common::uwhportal::schedule::{Event, EventId};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EventStore {
    /// Events fetched from the UWH Portal. `None` until a list has arrived,
    /// which the UI shows as "loading"; `Some` but empty is a portal that has
    /// no events to offer, which is a different thing and reads differently.
    portal: Option<BTreeMap<EventId, Event>>,
    /// The event named in the custom site's URL, once adopted. One, not a map:
    /// a custom site names exactly one.
    custom: Option<Event>,
}

impl EventStore {
    /// The events `source` offers for selection, or `None` when it has nothing
    /// to offer yet.
    ///
    /// Only the portal offers a list. A custom site's event is not chosen from
    /// anything — it is named in the URL — so `Custom` answers `None` and the
    /// SITE row takes the event picker's place on the page. `Manual` fetches
    /// nothing at all.
    pub(crate) fn selectable(&self, source: GameSource) -> Option<&BTreeMap<EventId, Event>> {
        match source {
            GameSource::Portal => self.portal.as_ref(),
            GameSource::Custom | GameSource::Manual => None,
        }
    }

    /// The data held for `id` under `source`, or `None` when `id` is not an
    /// event that source has.
    ///
    /// Resolution is by source and not by "whichever store holds the id":
    /// a custom site and the portal can legitimately use the same event id, and
    /// answering from the wrong one is the whole class of bug this type exists
    /// to prevent. `Manual` is not a site, so it holds nothing.
    pub(crate) fn get(&self, source: GameSource, id: &EventId) -> Option<&Event> {
        match source {
            GameSource::Portal => self.portal.as_ref()?.get(id),
            GameSource::Custom => self.custom.as_ref().filter(|event| event.id == *id),
            GameSource::Manual => None,
        }
    }

    /// As `get`, for the reply handlers that store teams and schedules into an
    /// event. Crossing sources is refused here too, so a reply that arrives
    /// after the operator has switched source lands nowhere and is logged,
    /// rather than being written into the other site's event.
    pub(crate) fn get_mut(&mut self, source: GameSource, id: &EventId) -> Option<&mut Event> {
        match source {
            GameSource::Portal => self.portal.as_mut()?.get_mut(id),
            GameSource::Custom => self.custom.as_mut().filter(|event| event.id == *id),
            GameSource::Manual => None,
        }
    }

    /// Whether `id` names an event `source` actually holds.
    ///
    /// This is the test that keeps one source's selection from being committed
    /// under the other: a staged custom-site event satisfies every completeness
    /// check the editor makes, so without this APPLY would commit it as a
    /// portal selection. `None` — nothing selected — is not owned by anyone.
    pub(crate) fn owns(&self, source: GameSource, id: Option<&EventId>) -> bool {
        id.is_some_and(|id| self.get(source, id).is_some())
    }

    /// Install a freshly-fetched portal list, replacing any previous one so an
    /// event deleted on the portal does not linger. The custom site's event is
    /// untouched: no portal fetch knows anything about it.
    pub(crate) fn set_portal_list(&mut self, events: BTreeMap<EventId, Event>) {
        self.portal = Some(events);
    }

    /// Adopt the event named in the custom site's URL.
    ///
    /// Adopting the event already held is a refresh, not a reset: the teams and
    /// schedule that have arrived for it are kept, because this is called again
    /// on every APPLY while CUSTOM is in use. A different id replaces it
    /// outright — a different address is a different event.
    pub(crate) fn adopt_custom(&mut self, event: Event) {
        match self.custom.as_ref() {
            Some(held) if held.id == event.id => {}
            _ => self.custom = Some(event),
        }
    }

    /// Whether a portal list has arrived. Distinguishes "still loading" from
    /// "the portal has no events", which the event tile renders differently.
    pub(crate) fn portal_list_loaded(&self) -> bool {
        self.portal.is_some()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use uwh_common::uwhportal::schedule::DateRange;

    fn event(partial: &str) -> Event {
        // `from_partial` is infallible — it only prefixes `events/`. The
        // length rule lives in `from_full`, which these tests do not use.
        let id = EventId::from_partial(partial);
        Event {
            id: id.clone(),
            name: format!("Event {partial}"),
            slug: String::new(),
            date_range: DateRange {
                start: time::OffsetDateTime::UNIX_EPOCH,
                end: time::OffsetDateTime::UNIX_EPOCH,
            },
            teams: None,
            schedule: None,
            courts: None,
        }
    }

    fn portal_list(store: &mut EventStore, partials: &[&str]) {
        store.set_portal_list(
            partials
                .iter()
                .map(|p| {
                    let e = event(p);
                    (e.id.clone(), e)
                })
                .collect(),
        );
    }

    /// The whole point of the type: a custom site's event is not one of the
    /// portal's, so the portal's picker must not be able to see it.
    #[test]
    fn a_custom_event_is_not_selectable_under_portal() {
        let mut store = EventStore::default();
        portal_list(&mut store, &["evt-A", "evt-B"]);
        store.adopt_custom(event("mine-1"));

        let selectable = store.selectable(GameSource::Portal).unwrap();
        assert_eq!(selectable.len(), 2);
        assert!(!selectable.contains_key(&EventId::from_partial("mine-1")));
    }

    /// A custom site serves no list at all — its event is named in the URL, so
    /// there is nothing to pick from.
    #[test]
    fn custom_offers_no_selectable_list() {
        let mut store = EventStore::default();
        store.adopt_custom(event("mine-1"));
        assert!(store.selectable(GameSource::Custom).is_none());
    }

    #[test]
    fn manual_offers_no_selectable_list() {
        let mut store = EventStore::default();
        portal_list(&mut store, &["evt-A"]);
        assert!(store.selectable(GameSource::Manual).is_none());
    }

    /// `None` before any list has arrived is distinct from `Some(empty)`, which
    /// is a portal that really has no events. The UI renders the first as
    /// "loading" and the second as an empty picker.
    #[test]
    fn portal_list_is_none_until_one_arrives() {
        let mut store = EventStore::default();
        assert!(store.selectable(GameSource::Portal).is_none());
        assert!(!store.portal_list_loaded());
        store.set_portal_list(BTreeMap::new());
        assert!(store.selectable(GameSource::Portal).is_some());
        assert!(store.portal_list_loaded());
    }

    /// Resolution is by source, not by "whichever store happens to hold the
    /// id". Two sites can legitimately use the same event id.
    #[test]
    fn get_resolves_only_within_the_named_source() {
        let mut store = EventStore::default();
        portal_list(&mut store, &["evt-A"]);
        store.adopt_custom(event("mine-1"));

        let portal_id = EventId::from_partial("evt-A");
        let custom_id = EventId::from_partial("mine-1");

        assert!(store.get(GameSource::Portal, &portal_id).is_some());
        assert!(store.get(GameSource::Portal, &custom_id).is_none());
        assert!(store.get(GameSource::Custom, &custom_id).is_some());
        assert!(store.get(GameSource::Custom, &portal_id).is_none());
        assert!(store.get(GameSource::Manual, &portal_id).is_none());
    }

    /// The same id held by both sources resolves to the right one for each.
    #[test]
    fn a_shared_id_resolves_per_source() {
        let mut store = EventStore::default();
        let mut portal_copy = event("dup-1");
        portal_copy.name = "portal copy".to_string();
        store.set_portal_list(
            [(portal_copy.id.clone(), portal_copy)]
                .into_iter()
                .collect(),
        );
        let mut custom_copy = event("dup-1");
        custom_copy.name = "custom copy".to_string();
        store.adopt_custom(custom_copy);

        let id = EventId::from_partial("dup-1");
        assert_eq!(
            store.get(GameSource::Portal, &id).unwrap().name,
            "portal copy"
        );
        assert_eq!(
            store.get(GameSource::Custom, &id).unwrap().name,
            "custom copy"
        );
    }

    /// `owns` is the test that keeps one source's selection from being
    /// committed under the other.
    #[test]
    fn owns_rejects_the_other_sources_selection_and_nothing_selected() {
        let mut store = EventStore::default();
        portal_list(&mut store, &["evt-A"]);
        store.adopt_custom(event("mine-1"));

        let portal_id = EventId::from_partial("evt-A");
        let custom_id = EventId::from_partial("mine-1");

        assert!(store.owns(GameSource::Portal, Some(&portal_id)));
        assert!(!store.owns(GameSource::Portal, Some(&custom_id)));
        assert!(!store.owns(GameSource::Portal, None));
        assert!(store.owns(GameSource::Custom, Some(&custom_id)));
        assert!(!store.owns(GameSource::Custom, Some(&portal_id)));
    }

    /// A fresh portal list replaces the previous one — an event deleted on the
    /// portal must not linger — but it must not disturb the custom site's
    /// event, which no portal fetch knows anything about.
    #[test]
    fn a_new_portal_list_replaces_the_old_and_spares_the_custom_event() {
        let mut store = EventStore::default();
        portal_list(&mut store, &["evt-A", "evt-B"]);
        store.adopt_custom(event("mine-1"));

        portal_list(&mut store, &["evt-C"]);

        let selectable = store.selectable(GameSource::Portal).unwrap();
        assert_eq!(selectable.len(), 1);
        assert!(selectable.contains_key(&EventId::from_partial("evt-C")));
        assert!(
            store
                .get(GameSource::Custom, &EventId::from_partial("mine-1"))
                .is_some()
        );
    }

    /// Re-adopting the same address must not throw away teams or the schedule
    /// that have already arrived for it — `adopt_custom_event` is called again
    /// on every APPLY while CUSTOM is in use.
    #[test]
    fn re_adopting_the_same_event_keeps_data_already_stored() {
        let mut store = EventStore::default();
        store.adopt_custom(event("mine-1"));
        let id = EventId::from_partial("mine-1");
        store.get_mut(GameSource::Custom, &id).unwrap().courts = Some(vec!["Court 1".to_string()]);

        store.adopt_custom(event("mine-1"));

        assert_eq!(
            store.get(GameSource::Custom, &id).unwrap().courts,
            Some(vec!["Court 1".to_string()])
        );
    }

    /// A different address is a different event, so nothing carries over.
    #[test]
    fn adopting_a_different_event_replaces_the_old_one() {
        let mut store = EventStore::default();
        store.adopt_custom(event("mine-1"));
        store
            .get_mut(GameSource::Custom, &EventId::from_partial("mine-1"))
            .unwrap()
            .courts = Some(vec!["Court 1".to_string()]);

        store.adopt_custom(event("mine-2"));

        assert!(
            store
                .get(GameSource::Custom, &EventId::from_partial("mine-1"))
                .is_none()
        );
        assert_eq!(
            store
                .get(GameSource::Custom, &EventId::from_partial("mine-2"))
                .unwrap()
                .courts,
            None
        );
    }

    /// The store-into path used by `RecvTeamsList` and `RecvSchedule` must not
    /// be able to write a reply into the other source's event.
    #[test]
    fn get_mut_will_not_cross_sources() {
        let mut store = EventStore::default();
        portal_list(&mut store, &["evt-A"]);
        store.adopt_custom(event("mine-1"));
        assert!(
            store
                .get_mut(GameSource::Portal, &EventId::from_partial("mine-1"))
                .is_none()
        );
        assert!(
            store
                .get_mut(GameSource::Custom, &EventId::from_partial("evt-A"))
                .is_none()
        );
    }
}
