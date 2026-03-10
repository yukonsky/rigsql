pub mod rule;
pub mod utils;
mod violation;

pub mod capitalisation;
pub mod layout;
pub mod convention;
pub mod aliasing;

pub use rule::{CrawlType, Rule, RuleContext, RuleGroup, apply_fixes};
pub use violation::{LintViolation, Severity, SourceEdit};

/// Returns all default rules.
pub fn default_rules() -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();

    // Capitalisation
    rules.push(Box::new(capitalisation::cp01::RuleCP01::default()));
    rules.push(Box::new(capitalisation::cp02::RuleCP02::default()));
    rules.push(Box::new(capitalisation::cp03::RuleCP03::default()));
    rules.push(Box::new(capitalisation::cp04::RuleCP04::default()));
    rules.push(Box::new(capitalisation::cp05::RuleCP05::default()));

    // Layout
    rules.push(Box::new(layout::lt01::RuleLT01::default()));
    rules.push(Box::new(layout::lt02::RuleLT02::default()));
    rules.push(Box::new(layout::lt03::RuleLT03::default()));
    rules.push(Box::new(layout::lt04::RuleLT04::default()));
    rules.push(Box::new(layout::lt05::RuleLT05::default()));
    rules.push(Box::new(layout::lt06::RuleLT06::default()));
    rules.push(Box::new(layout::lt07::RuleLT07::default()));
    rules.push(Box::new(layout::lt09::RuleLT09::default()));
    rules.push(Box::new(layout::lt10::RuleLT10::default()));
    rules.push(Box::new(layout::lt11::RuleLT11::default()));
    rules.push(Box::new(layout::lt12::RuleLT12::default()));
    rules.push(Box::new(layout::lt13::RuleLT13::default()));
    rules.push(Box::new(layout::lt14::RuleLT14::default()));
    rules.push(Box::new(layout::lt15::RuleLT15::default()));

    // Convention
    rules.push(Box::new(convention::cv01::RuleCV01::default()));
    rules.push(Box::new(convention::cv02::RuleCV02::default()));
    rules.push(Box::new(convention::cv03::RuleCV03::default()));
    rules.push(Box::new(convention::cv04::RuleCV04::default()));
    rules.push(Box::new(convention::cv05::RuleCV05::default()));
    rules.push(Box::new(convention::cv06::RuleCV06::default()));
    rules.push(Box::new(convention::cv07::RuleCV07::default()));
    rules.push(Box::new(convention::cv08::RuleCV08::default()));
    rules.push(Box::new(convention::cv09::RuleCV09::default()));
    rules.push(Box::new(convention::cv10::RuleCV10::default()));
    rules.push(Box::new(convention::cv11::RuleCV11::default()));
    rules.push(Box::new(convention::cv12::RuleCV12::default()));

    // Aliasing
    rules.push(Box::new(aliasing::al01::RuleAL01::default()));
    rules.push(Box::new(aliasing::al02::RuleAL02::default()));
    rules.push(Box::new(aliasing::al03::RuleAL03::default()));
    rules.push(Box::new(aliasing::al04::RuleAL04::default()));
    rules.push(Box::new(aliasing::al05::RuleAL05::default()));
    rules.push(Box::new(aliasing::al07::RuleAL07::default()));

    rules
}
