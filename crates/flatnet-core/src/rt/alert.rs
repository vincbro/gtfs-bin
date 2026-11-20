use crate::{
    models::{Optional, RouteIndex, RouteType, StopIndex, TripIndex},
    rt::Text,
};

/// A service alert, with the entities it affects resolved against the static data.
#[derive(Debug, Clone, PartialEq)]
pub struct Alert {
    /// The id of the feed entity, which a later alert with the same id replaces.
    pub id: Box<str>,
    /// When the alert applies, as POSIX times. No periods means it always applies.
    pub active_periods: Box<[ActivePeriod]>,
    pub informed: Box<[InformedEntity]>,
    pub cause: Cause,
    pub effect: Effect,
    pub severity: Severity,
    pub header: Box<str>,
    pub description: Box<str>,
    pub url: Box<str>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    #[default]
    Unknown,
    Other,
    TechnicalProblem,
    Strike,
    Demonstration,
    Accident,
    Holiday,
    Weather,
    Maintenance,
    Construction,
    PoliceActivity,
    MedicalEmergency,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    NoService,
    ReducedService,
    SignificantDelays,
    Detour,
    AdditionalService,
    ModifiedService,
    Other,
    #[default]
    Unknown,
    StopMoved,
    NoEffect,
    AccessibilityIssue,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    #[default]
    Unknown,
    Info,
    Warning,
    Severe,
}

/// A span of time, where a missing end means it never ends and a missing start means it already
/// began.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ActivePeriod {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

/// One thing an alert is about. Every field that is set must match, and one that isn't set matches
/// anything.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InformedEntity {
    pub agency_id: Option<String>,
    pub route: Optional<RouteIndex>,
    pub route_type: Optional<RouteType>,
    pub trip: Optional<TripIndex>,
    pub stop: Optional<StopIndex>,
}

impl Alert {
    /// Returns whether the alert applies at a POSIX time.
    #[must_use]
    pub fn is_active_at(&self, time: u64) -> bool {
        self.active_periods.is_empty()
            || self.active_periods.iter().any(|period| {
                period.start.is_none_or(|start| start <= time)
                    && period.end.is_none_or(|end| time < end)
            })
    }

    /// Returns whether the alert is about the stop.
    #[must_use]
    pub fn affects_stop(&self, stop: StopIndex) -> bool {
        self.informed
            .iter()
            .any(|entity| entity.stop.as_option() == Some(stop))
    }

    /// Returns whether the alert is about the route.
    #[must_use]
    pub fn affects_route(&self, route: RouteIndex) -> bool {
        self.informed
            .iter()
            .any(|entity| entity.route.as_option() == Some(route))
    }

    /// Returns whether the alert is about the trip.
    #[must_use]
    pub fn affects_trip(&self, trip: TripIndex) -> bool {
        self.informed
            .iter()
            .any(|entity| entity.trip.as_option() == Some(trip))
    }
}

/// Picks the text in the preferred language, or the first one if there is no such translation.
pub(super) fn translated(text: Option<&Text>, language: Option<&str>) -> Box<str> {
    let Some(text) = text else {
        return Box::default();
    };
    let preferred = language.and_then(|language| {
        text.translations
            .iter()
            .find(|translation| translation.language.as_deref() == Some(language))
    });
    preferred
        .or_else(|| text.translations.first())
        .map_or_else(Box::default, |translation| {
            Box::from(translation.text.as_str())
        })
}
