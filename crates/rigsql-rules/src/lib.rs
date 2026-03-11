pub mod rule;
pub mod utils;
mod violation;

#[cfg(test)]
pub(crate) mod test_utils;

pub mod aliasing;
pub mod capitalisation;
pub mod convention;
pub mod layout;

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
    ]
}
