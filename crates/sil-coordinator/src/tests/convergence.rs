use super::support::*;
use crate::Coordinator;
use sil_ledger::TaskStatus;
use sil_reputation::Outcome;
use sil_types::PublicKey;

#[test]
fn members_wait_for_the_observation_window_before_voting() {
    let worker = ident(60);
    let mut world = World::with_window(1_000);
    world.claim(&worker, 1);
    assert_eq!(
        world.drive(),
        0,
        "le claim est trop récent : on attend les autres"
    );
    world.now += 1_500;
    assert!(world.drive() > 0, "fenêtre écoulée : les membres votent");
    assert_eq!(world.task().attempts[0].claimant, worker.public());
}

#[test]
fn a_member_follows_a_vote_already_cast_even_if_another_claim_ranks_higher() {
    let (newcomer, veteran) = (ident(60), ident(61));
    let mut world = World::new();
    let insiders: Vec<PublicKey> = world.coordinators.iter().map(Coordinator::public).collect();
    for _ in 0..5 {
        world.reputation.observe(
            &ident(100).public(),
            &veteran.public(),
            &insiders,
            Outcome::Completed { reward: 10 },
            world.now,
        );
    }
    world.claim(&newcomer, 1);
    // Le premier membre ne voit que le claim du nouveau et vote pour lui.
    let first =
        world.coordinators[0].decide(&world.ledger, &world.reputation, &world.job, world.now);
    assert_eq!(first.len(), 1);
    world.now += 1;
    world
        .ledger
        .append_local(
            &ident(10),
            world.job,
            first.into_iter().next().unwrap(),
            world.now,
        )
        .unwrap();
    // Le vétéran réclame ensuite et se classe mieux : le deuxième membre suit quand même
    // le vote déjà exprimé, sinon la décision serait coupée en deux et jamais atteinte.
    world.claim(&veteran, 1);
    world.drive();
    assert_eq!(world.task().attempts[0].claimant, newcomer.public());
}

fn leader_of(world: &World) -> usize {
    crate::leader_index(&world.job, 0, 1, world.coordinators.len())
}

#[test]
fn non_leaders_wait_for_the_leader_and_then_vote_for_its_claim() {
    let (first, second) = (ident(60), ident(61));
    let mut world = World::with_tuning(0, 5_000);
    world.claim(&first, 1);
    world.claim(&second, 1);
    let leader = leader_of(&world);
    // Les non-chefs attendent le vote du chef : sans lui, aucun vote.
    assert_eq!(
        world.drive_except(Some(leader)),
        0,
        "les non-chefs attendent"
    );
    // Puis le chef vote seul : une voix, la sienne. Quel que soit le chef tiré du job, l'ordre
    // de passage ne change rien.
    assert_eq!(world.drive_only(leader), 1, "un seul vote : celui du chef");
    assert_eq!(world.task().status, TaskStatus::Open, "1 voix < seuil 2");
    let chosen = world.ledger.claim_votes(&world.job, 0, 1)[0];
    assert_eq!(chosen.0, world.coordinators[leader].public());
    // Les autres votent ensuite pour le même claim : décision atteinte.
    world.drive();
    assert_eq!(world.task().status, TaskStatus::Claimed);
    assert_eq!(
        world
            .ledger
            .claim_votes(&world.job, 0, 1)
            .iter()
            .filter(|(_, c)| *c == chosen.1)
            .count(),
        2
    );
}

#[test]
fn a_silent_leader_is_replaced_once_the_wait_is_over() {
    let worker = ident(60);
    let mut world = World::with_tuning(0, 5_000);
    world.claim(&worker, 1);
    let leader = leader_of(&world);
    assert_eq!(world.drive_except(Some(leader)), 0, "ils attendent le chef");
    world.now += 6_000;
    assert!(
        world.drive_except(Some(leader)) > 0,
        "attente écoulée : ils votent seuls"
    );
    assert_eq!(
        world.task().status,
        TaskStatus::Claimed,
        "2 membres sur 3 suffisent"
    );
}
