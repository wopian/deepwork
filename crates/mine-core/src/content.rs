//! Validate content references and prove progression can reach every module.
use crate::{
    materials, recipes, requirements, sites, traces, Material, Recipe, UpgradeRequirement,
};
use std::collections::BTreeSet;
pub fn validate() -> Result<(), String> {
    crate::pacing::validate()?;
    let cat = materials();
    if cat
        .iter()
        .enumerate()
        .any(|(i, m)| m.id != i || m.price == 0 || m.tier > 5)
    {
        return Err("Invalid mineral catalogue".into());
    }
    for site in sites() {
        if !site.hardness.is_finite()
            || site.hardness <= 0.
            || !site.haul.is_finite()
            || site.haul <= 0.
            || site.focus.iter().any(|id| *id >= cat.len())
        {
            return Err("Invalid site profile".into());
        }
    }
    for rule in traces() {
        if rule.feed >= cat.len() || rule.permille == 0 || rule.permille > 1000 {
            return Err("Invalid trace feed".into());
        }
    }
    for feed in &cat {
        if traces()
            .iter()
            .filter(|t| t.feed == feed.id)
            .map(|t| t.permille)
            .sum::<u64>()
            > 1000
        {
            return Err("Trace recovery exceeds feed".into());
        }
    }
    progression(&cat, recipes(), requirements())
}
fn progression(
    cat: &[Material],
    recipes: &[Recipe],
    upgrades: &[UpgradeRequirement],
) -> Result<(), String> {
    let ids: BTreeSet<_> = upgrades.iter().map(|u| u.id.as_str()).collect();
    if ids.len() != upgrades.len()
        || recipes.iter().map(|r| &r.id).collect::<BTreeSet<_>>().len() != recipes.len()
    {
        return Err("Duplicate content identifier".into());
    }
    for u in upgrades {
        if (!u.requires.is_empty() && !ids.contains(u.requires.as_str()))
            || u.inputs.values().any(|n| *n == 0)
        {
            return Err(format!("Invalid upgrade {}", u.id));
        }
    }
    for r in recipes {
        if !ids.contains(r.building.as_str())
            || r.inputs.is_empty()
            || r.inputs.values().any(|n| *n == 0)
        {
            return Err(format!("Invalid recipe {}", r.id));
        }
    }
    let mut buildings = BTreeSet::new();
    let mut products = BTreeSet::new();
    loop {
        let before = (buildings.len(), products.len());
        for u in upgrades {
            if (u.requires.is_empty() || buildings.contains(u.requires.as_str()))
                && u.inputs.keys().all(|p| products.contains(p))
            {
                buildings.insert(u.id.as_str());
            }
        }
        let tier = if buildings.contains("ventilation") {
            5
        } else if buildings.contains("pump") {
            3
        } else if buildings.contains("supports") {
            2
        } else if buildings.contains("shaft") {
            1
        } else {
            0
        };
        for m in cat.iter().filter(|m| m.tier <= tier) {
            let module = match m.family.as_str() {
                "physical" => "",
                "furnace" | "industrial" => "furnace",
                "sulfide" | "chemical" | "electrolytic" => "chemical",
                _ => "trace",
            };
            if module.is_empty() || buildings.contains(module) {
                products.insert(if m.product == "aluminium" {
                    "alumina".into()
                } else {
                    m.product.clone()
                });
                for rule in traces().iter().filter(|t| t.feed == m.id) {
                    products.insert(format!("{}_residue", rule.output));
                }
            }
        }
        for recipe in recipes {
            if buildings.contains(recipe.building.as_str())
                && recipe.inputs.keys().all(|p| products.contains(p))
            {
                products.insert(recipe.output.clone());
            }
        }
        if before == (buildings.len(), products.len()) {
            break;
        }
    }
    for u in upgrades {
        if !buildings.contains(u.id.as_str()) {
            return Err(format!(
                "Inaccessible upgrade or dependency cycle: {}",
                u.id
            ));
        }
    }
    for r in recipes {
        if !products.contains(&r.output) {
            return Err(format!("Inaccessible recipe: {}", r.id));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shipped_campaign_has_reachable_unlocks() {
        validate().unwrap();
    }
    #[test]
    fn equipment_output_cycle_is_rejected() {
        let mut upgrades = requirements().to_vec();
        upgrades
            .iter_mut()
            .find(|u| u.id == "furnace")
            .unwrap()
            .requires = "steelworks".into();
        assert!(progression(&materials(), recipes(), &upgrades).is_err());
    }
}
