//! Invalidation events for undo and redo, computed by pointer comparison of snapshots.

use mc_score::{DomainEvent, Score};

pub(crate) fn changes(old: &Score, new: &Score) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    for t in old.tracks() {
        if new.track(t.id()).is_none() {
            events.push(DomainEvent::TrackRemoved { track: t.id() });
        }
    }
    let structure_changed = !new.shares_master_bars_with(old);
    if structure_changed {
        events.push(DomainEvent::MasterBarsChanged {
            bars: 0..new.master_bar_count(),
        });
    }
    if !new.shares_tempo_map_with(old) {
        events.push(DomainEvent::TempoMapChanged);
    }
    let old_order: Vec<_> = old.tracks().map(|t| t.id()).collect();
    let new_order: Vec<_> = new.tracks().map(|t| t.id()).collect();
    for t in new.tracks() {
        let Some(o) = old.track(t.id()) else {
            events.push(DomainEvent::TrackAdded { track: t.id() });
            continue;
        };
        if t.name() != o.name() || t.instrument() != o.instrument() {
            events.push(DomainEvent::TrackChanged { track: t.id() });
        }
        let len = t.bars().len();
        if structure_changed || len != o.bars().len() {
            events.push(DomainEvent::BarContentChanged {
                track: t.id(),
                bars: 0..len,
            });
            continue;
        }
        let changed: Vec<usize> = (0..len).filter(|&i| !t.shares_bar_with(o, i)).collect();
        if let (Some(&first), Some(&last)) = (changed.first(), changed.last()) {
            events.push(DomainEvent::BarContentChanged {
                track: t.id(),
                bars: first..last + 1,
            });
        }
    }
    let common: Vec<_> = new_order
        .iter()
        .filter(|id| old_order.contains(id))
        .collect();
    let common_old: Vec<_> = old_order
        .iter()
        .filter(|id| new_order.contains(id))
        .collect();
    if common != common_old {
        events.push(DomainEvent::TracksReordered);
    }
    events
}
