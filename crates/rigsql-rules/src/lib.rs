pub mod rule;
pub mod utils;
mod violation;

#[cfg(test)]
pub(crate) mod test_utils;

pub mod aliasing;
pub mod ambiguous;
pub mod capitalisation;
pub mod convention;
pub mod layout;
pub mod references;
pub mod structure;
pub mod tsql;

pub use rule::{apply_fixes, CrawlType, Rule, RuleContext, RuleGroup};
pub use violation::{LintViolation, Severity, SourceEdit};

/// Returns all default rules.
pub fn default_rules() -> Vec<Box<dyn Rule>> {
    vec![
        // Capitalisation
        Box::new(capitalisation::cp01::RuleCP01::default()),
        Box::new(capitalisation::cp02::RuleCP02::default()),
        Box::new(capitalisation::cp03::RuleCP03),
        Box::new(capitalisation::cp04::RuleCP04),
        Box::new(capitalisation::cp05::RuleCP05::default()),
        // Layout
        Box::new(layout::lt01::RuleLT01),
        Box::new(layout::lt02::RuleLT02::default()),
        Box::new(layout::lt03::RuleLT03),
        Box::new(layout::lt04::RuleLT04::default()),
        Box::new(layout::lt05::RuleLT05::default()),
        Box::new(layout::lt06::RuleLT06),
        Box::new(layout::lt07::RuleLT07),
        Box::new(layout::lt08::RuleLT08),
        Box::new(layout::lt09::RuleLT09),
        Box::new(layout::lt10::RuleLT10),
        Box::new(layout::lt11::RuleLT11),
        Box::new(layout::lt12::RuleLT12),
        Box::new(layout::lt13::RuleLT13),
        Box::new(layout::lt14::RuleLT14),
        Box::new(layout::lt15::RuleLT15),
        // Convention
        Box::new(convention::cv01::RuleCV01::default()),
        Box::new(convention::cv02::RuleCV02),
        Box::new(convention::cv03::RuleCV03),
        Box::new(convention::cv04::RuleCV04),
        Box::new(convention::cv05::RuleCV05),
        Box::new(convention::cv06::RuleCV06),
        Box::new(convention::cv07::RuleCV07),
        Box::new(convention::cv08::RuleCV08),
        Box::new(convention::cv09::RuleCV09::default()),
        Box::new(convention::cv10::RuleCV10),
        Box::new(convention::cv11::RuleCV11),
        Box::new(convention::cv12::RuleCV12),
        // Aliasing
        Box::new(aliasing::al01::RuleAL01),
        Box::new(aliasing::al02::RuleAL02),
        Box::new(aliasing::al03::RuleAL03),
        Box::new(aliasing::al04::RuleAL04),
        Box::new(aliasing::al05::RuleAL05),
        Box::new(aliasing::al06::RuleAL06),
        Box::new(aliasing::al07::RuleAL07::default()),
        Box::new(aliasing::al08::RuleAL08),
        Box::new(aliasing::al09::RuleAL09),
        // Ambiguous
        Box::new(ambiguous::am01::RuleAM01),
        Box::new(ambiguous::am02::RuleAM02),
        Box::new(ambiguous::am03::RuleAM03),
        Box::new(ambiguous::am04::RuleAM04),
        Box::new(ambiguous::am05::RuleAM05),
        Box::new(ambiguous::am06::RuleAM06),
        Box::new(ambiguous::am07::RuleAM07),
        Box::new(ambiguous::am08::RuleAM08),
        Box::new(ambiguous::am09::RuleAM09),
        // References
        Box::new(references::rf01::RuleRF01),
        Box::new(references::rf02::RuleRF02),
        Box::new(references::rf03::RuleRF03),
        Box::new(references::rf04::RuleRF04),
        Box::new(references::rf05::RuleRF05),
        Box::new(references::rf06::RuleRF06),
        // Structure
        Box::new(structure::st01::RuleST01),
        Box::new(structure::st02::RuleST02),
        Box::new(structure::st03::RuleST03),
        Box::new(structure::st04::RuleST04),
        Box::new(structure::st05::RuleST05),
        Box::new(structure::st06::RuleST06),
        Box::new(structure::st07::RuleST07),
        Box::new(structure::st08::RuleST08),
        Box::new(structure::st09::RuleST09),
        Box::new(structure::st10::RuleST10),
        Box::new(structure::st11::RuleST11),
        Box::new(structure::st12::RuleST12),
        // TSQL
        Box::new(tsql::tq01::RuleTQ01),
        Box::new(tsql::tq02::RuleTQ02),
        Box::new(tsql::tq03::RuleTQ03),
    ]
}
