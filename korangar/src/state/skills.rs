use std::sync::Arc;

use hashbrown::HashMap;
use korangar_interface::element::StateElement;
use ragnarok_packets::{AttackRange, JobId, SkillId, SkillInformation, SkillLevel, SkillType};
use rust_state::{Path, PathExt, RustState, Selector};

use crate::loaders::Sprite;
use crate::state::ClientState;
use crate::world::{Actions, Library, SkillListKey, SkillListRequirements, SpriteAnimationState};

pub struct LearnedSkillPath<A, B> {
    learnable_skill_path: A,
    skills_path: B,
}

impl<A, B> LearnedSkillPath<A, B> {
    pub fn new(learnable_skill_path: A, skills_path: B) -> Self {
        Self {
            learnable_skill_path,
            skills_path,
        }
    }
}

impl<A, B> Clone for LearnedSkillPath<A, B>
where
    A: Clone,
    B: Clone,
{
    fn clone(&self) -> Self {
        Self {
            learnable_skill_path: self.learnable_skill_path.clone(),
            skills_path: self.skills_path.clone(),
        }
    }
}

impl<A, B> Copy for LearnedSkillPath<A, B>
where
    A: Copy,
    B: Copy,
{
}

impl<A, B> Path<ClientState, LearnedSkill, false> for LearnedSkillPath<A, B>
where
    A: Path<ClientState, LearnableSkill, false>,
    B: Path<ClientState, Vec<LearnedSkill>>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a LearnedSkill> {
        let learnable_skill = self.learnable_skill_path.follow(state)?;
        let learnable_skill_id = learnable_skill.skill_id;

        let skills = self.skills_path.follow_safe(state);

        skills.iter().find(|skill| skill.skill_id == learnable_skill_id)
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut LearnedSkill> {
        let learnable_skill = self.learnable_skill_path.follow(state)?;
        let learnable_skill_id = learnable_skill.skill_id;

        let skills = self.skills_path.follow_mut_safe(state);

        skills.iter_mut().find(|skill| skill.skill_id == learnable_skill_id)
    }
}

impl<A, B> Selector<ClientState, LearnedSkill, false> for LearnedSkillPath<A, B>
where
    A: Path<ClientState, LearnableSkill, false>,
    B: Path<ClientState, Vec<LearnedSkill>>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a LearnedSkill> {
        self.follow(state)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, RustState, StateElement)]
pub enum SkillAcquisition {
    Job,
    Quest,
    SoulLink,
}

#[derive(Clone, Debug, RustState, StateElement)]
pub struct LearnableSkill {
    pub skill_id: SkillId,
    pub maximum_level: SkillLevel,
    pub file_name: String,
    pub skill_name: String,
    pub can_select_level: bool,
    pub acquisition: SkillAcquisition,
    // TODO: Unhide this
    #[hidden_element]
    pub required_skills: HashMap<SkillId, SkillLevel>,
    #[hidden_element]
    pub required_for_skills: HashMap<SkillId, SkillLevel>,
    // TODO: Unhide this
    #[hidden_element]
    pub sprite: Option<Arc<Sprite>>,
    // TODO: Unhide this
    #[hidden_element]
    pub actions: Option<Arc<Actions>>,
    pub animation_state: SpriteAnimationState,
}

#[derive(Clone, Debug, RustState, StateElement)]
pub struct LearnedSkill {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub skill_type: SkillType,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    pub skill_name: String,
    pub upgradable: bool,
}

impl LearnedSkill {
    pub fn new(
        SkillInformation {
            skill_id,
            skill_type,
            skill_level,
            spell_point_cost,
            attack_range,
            skill_name,
            upgradable,
        }: SkillInformation,
    ) -> Self {
        LearnedSkill {
            skill_id,
            skill_level,
            skill_type,
            spell_point_cost,
            attack_range,
            skill_name,
            upgradable: upgradable != 0,
        }
    }
}

#[derive(Debug, Clone, RustState, StateElement)]
pub struct SkillTabLayout {
    pub name: String,
    #[hidden_element]
    pub skills: HashMap<usize, LearnableSkill>,
}

#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct SkillTreeLayout {
    pub tabs: Vec<SkillTabLayout>,
}

#[derive(Default, RustState, StateElement)]
pub struct SkillTree {
    layout: SkillTreeLayout,
    skills: Vec<LearnedSkill>,
}

impl SkillTree {
    /// `(skill id, learned level)` for every skill the character has.
    pub fn learned_levels(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        self.skills.iter().map(|skill| (skill.skill_id.0, skill.skill_level.0))
    }

    /// Remove character-specific skill data while retaining globally cached
    /// SPR/ACT resources in the loaders.
    pub fn clear(&mut self) {
        self.layout.tabs.clear();
        self.skills.clear();
    }

    pub fn upsert_skill(&mut self, skill: LearnedSkill) {
        if let Some(existing) = self.skills.iter_mut().find(|existing| existing.skill_id == skill.skill_id) {
            *existing = skill;
        } else {
            self.skills.push(skill);
        }
    }

    pub fn update_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        spell_point_cost: u16,
        attack_range: AttackRange,
        upgradable: bool,
    ) {
        if let Some(skill) = self.skills.iter_mut().find(|skill| skill.skill_id == skill_id) {
            skill.skill_level = skill_level;
            skill.spell_point_cost = spell_point_cost;
            skill.attack_range = attack_range;
            skill.upgradable = upgradable;
        }
    }

    pub fn remove_skill(&mut self, skill_id: SkillId) {
        self.skills.retain(|skill| skill.skill_id != skill_id);
    }
}

pub fn bring_skill_to_level(
    collected_skill_points: &mut Vec<SkillId>,
    library: &Library,
    learned_skills: &[LearnedSkill],
    job_id: JobId,
    skill_id: SkillId,
    target_skill_level: SkillLevel,
    mut available_skill_points: usize,
) -> usize {
    let skill_requirements = library.get::<SkillListRequirements>(SkillListKey::with_job(job_id, skill_id));

    let current_skill_level = learned_skills
        .iter()
        .find(|skill| skill.skill_id == skill_id)
        .map(|skill| skill.skill_level.0)
        .unwrap_or_default()
        + collected_skill_points
            .iter()
            .filter(|pending_skill_level| **pending_skill_level == skill_id)
            .count() as u16;

    // Early return for met requirements.
    if current_skill_level >= target_skill_level.0 {
        return available_skill_points;
    }

    for (required_skill_id, required_skill_level) in &skill_requirements.required_skills {
        available_skill_points = bring_skill_to_level(
            collected_skill_points,
            library,
            learned_skills,
            job_id,
            *required_skill_id,
            *required_skill_level,
            available_skill_points,
        );

        // Early return when running out of skill points.
        if available_skill_points == 0 {
            return 0;
        }
    }

    let total_points_required = (target_skill_level.0 - current_skill_level) as usize;

    // Attempt to bring the skill to the required level.
    collected_skill_points.extend(std::iter::repeat_n(skill_id, total_points_required.min(available_skill_points)));

    available_skill_points.saturating_sub(total_points_required)
}

/// Novice Basic Skill. Once the character leaves the novice job, a full reset
/// keeps this skill and does not return its points. `@refundskill` does the
/// same.
pub const NOVICE_BASIC_SKILL_ID: SkillId = SkillId(1);

/// Job ids whose server map identity is still Novice: Novice, High Novice,
/// Baby.
const NOVICE_JOB_IDS: [u16; 3] = [0, 4001, 4023];

/// What the skill-tree minus does with one point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillPointRefund {
    /// Drop one locally queued point. The server has not been asked yet.
    UndoPending,
    /// Ask the server to return one spent point.
    AskServer,
}

/// Decide the skill-tree minus. Quest and soul-link skills are not spent
/// points. A queued point is undone before a spent point is refunded, so Apply
/// cannot put the point back. Basic Skill is only refundable on a novice job;
/// the server refuses everyone else.
pub fn skill_point_refund(
    acquisition: SkillAcquisition,
    skill_id: SkillId,
    learned_level: u16,
    pending_points: u16,
    job_id: Option<JobId>,
) -> Option<SkillPointRefund> {
    if acquisition != SkillAcquisition::Job {
        return None;
    }
    if pending_points > 0 {
        return Some(SkillPointRefund::UndoPending);
    }
    if learned_level == 0 {
        return None;
    }
    if skill_id == NOVICE_BASIC_SKILL_ID && !job_id.is_some_and(|job| NOVICE_JOB_IDS.contains(&job.0)) {
        return None;
    }
    Some(SkillPointRefund::AskServer)
}

/// Forget a cast level that is no longer strictly below the learned rank.
/// The map only stores a level when the player chose to cast below the rank
/// they have. A refund replaces the skill list and can leave a higher choice.
pub fn clamp_chosen_cast_levels(chosen: &mut HashMap<SkillId, SkillLevel>, learned: &[(SkillId, SkillLevel)]) {
    chosen.retain(|skill_id, level| {
        learned
            .iter()
            .any(|(id, learned_level)| *id == *skill_id && level.0 > 0 && level.0 < learned_level.0)
    });
}

/// One entry in the Auto Spell chooser: the skill to answer with, and a name
/// to show for it.
#[derive(Clone, Debug, RustState, StateElement)]
pub struct AutoSpellChoice {
    pub skill_id: SkillId,
    pub name: String,
}

#[cfg(test)]
mod skill_point_refund_tests {
    use ragnarok_packets::{JobId, SkillId, SkillLevel};

    use super::{SkillAcquisition, SkillPointRefund, clamp_chosen_cast_levels, skill_point_refund};

    fn refund(acquisition: SkillAcquisition, skill_id: u16, learned: u16, pending: u16, job: Option<u16>) -> Option<SkillPointRefund> {
        skill_point_refund(acquisition, SkillId(skill_id), learned, pending, job.map(JobId))
    }

    #[test]
    fn spent_job_skill_asks_the_server() {
        assert_eq!(
            refund(SkillAcquisition::Job, 5, 3, 0, Some(1)),
            Some(SkillPointRefund::AskServer)
        );
    }

    #[test]
    fn queued_point_is_undone_before_a_spent_point() {
        assert_eq!(
            refund(SkillAcquisition::Job, 5, 3, 1, Some(1)),
            Some(SkillPointRefund::UndoPending)
        );
        assert_eq!(
            refund(SkillAcquisition::Job, 5, 0, 2, Some(1)),
            Some(SkillPointRefund::UndoPending)
        );
    }

    #[test]
    fn quest_and_soul_link_skills_have_no_minus() {
        assert_eq!(refund(SkillAcquisition::Quest, 142, 1, 0, Some(0)), None);
        assert_eq!(refund(SkillAcquisition::SoulLink, 261, 1, 0, Some(1)), None);
        assert_eq!(refund(SkillAcquisition::Quest, 142, 1, 1, Some(0)), None);
    }

    #[test]
    fn unlearned_job_skill_has_no_minus() {
        assert_eq!(refund(SkillAcquisition::Job, 5, 0, 0, Some(1)), None);
    }

    #[test]
    fn basic_skill_refunds_only_for_a_novice() {
        assert_eq!(
            refund(SkillAcquisition::Job, 1, 9, 0, Some(0)),
            Some(SkillPointRefund::AskServer)
        );
        assert_eq!(
            refund(SkillAcquisition::Job, 1, 9, 0, Some(4001)),
            Some(SkillPointRefund::AskServer)
        );
        assert_eq!(
            refund(SkillAcquisition::Job, 1, 9, 0, Some(4023)),
            Some(SkillPointRefund::AskServer)
        );
        assert_eq!(refund(SkillAcquisition::Job, 1, 9, 0, Some(1)), None);
        assert_eq!(refund(SkillAcquisition::Job, 1, 9, 0, Some(23)), None);
        assert_eq!(refund(SkillAcquisition::Job, 1, 9, 0, None), None);
        assert_eq!(
            refund(SkillAcquisition::Job, 1, 9, 1, Some(1)),
            Some(SkillPointRefund::UndoPending)
        );
    }

    #[test]
    fn chosen_cast_level_drops_when_the_learned_rank_no_longer_contains_it() {
        let mut chosen = hashbrown::HashMap::new();
        chosen.insert(SkillId(5), SkillLevel(3));
        chosen.insert(SkillId(7), SkillLevel(5));
        chosen.insert(SkillId(8), SkillLevel(1));
        chosen.insert(SkillId(9), SkillLevel(0));

        clamp_chosen_cast_levels(&mut chosen, &[
            (SkillId(5), SkillLevel(5)),
            (SkillId(7), SkillLevel(4)),
            (SkillId(9), SkillLevel(3)),
        ]);

        assert_eq!(chosen.get(&SkillId(5)).copied(), Some(SkillLevel(3)));
        assert!(!chosen.contains_key(&SkillId(7)));
        assert!(!chosen.contains_key(&SkillId(8)));
        assert!(!chosen.contains_key(&SkillId(9)));
    }
}
