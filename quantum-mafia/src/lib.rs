#[derive(PartialEq, Eq, Clone, Debug)]
pub enum PersonClass {
    Villager,
    Mafiosi,
    Medic,
    Necromancer,
    Priest,
    Dead,
    Zombie,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum QuantumMafiaError {
    WrongLengthAction,
    WrongTime,
    BadPersonAssignments,
    WrongGameSize,
}

use QuantumMafiaError as QME;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Person(usize);

#[derive(Clone, Debug)]
pub struct DayAction(Person);

#[derive(Clone, Debug)]
pub struct NightAction(Person);

#[derive(Clone, Debug)]
pub struct DayActions {
    accused: Person,
    // True = lynch them
    votes: Vec<bool>,
}

#[derive(Clone, Debug)]
pub struct NightActions(Vec<NightAction>);

#[derive(Clone, Debug)]
struct Turn(NightActions, DayActions);

#[derive(Debug)]
pub struct SubGame {
    people: Vec<PersonClass>,
}

#[derive(Debug)]
pub struct QuantumMafia {
    subgames: Vec<SubGame>,
    history: Vec<Turn>,
    lastnight: Option<NightActions>,
    player_names: Vec<String>,
}

impl SubGame {
    pub fn new(people: Vec<PersonClass>) -> Result<SubGame, QME> {
        Ok(SubGame { people })
    }
}

impl QuantumMafia {
    pub fn new(player_names: Vec<String>, subgames: Vec<SubGame>) -> Result<QuantumMafia, QME> {
        for sg in subgames.iter() {
            if sg.people.len() != player_names.len() {
                return Err(QME::WrongGameSize);
            }
        }
        Ok(QuantumMafia {
               subgames,
               history: Vec::new(),
               lastnight: None,
               player_names,
           })
    }

    pub fn new_night_action(&self, votes: Vec<NightAction>) -> Result<NightActions, QME> {
        if self.subgames.len() != votes.len() {
            return Err(QME::WrongLengthAction);
        }
        Ok(NightActions(votes))
    }

    pub fn do_night(&mut self, a: NightActions) -> Result<(), QME> {
        if a.0.len() != self.subgames.len() {
            return Err(QME::WrongLengthAction);
        }
        if self.lastnight.is_some() {
            return Err(QME::WrongTime);
        }

        self.lastnight = Some(a.clone());

        // Apply the same rules to each subgame
        for thisgame in self.subgames.iter_mut() {

            enum MafiosiState {
                NotAgreeing,
                NoVotesYet,
                VoteFor(Person),
            }
            let mut mafiosi_state = MafiosiState::NoVotesYet;
            let mut medic_vote = None;
            let mut necromancer_vote = None;
            let mut priest_vote = None;

            // Gather the votes for each player
            for i in 0..a.0.len() {
                match thisgame.people[i] {
                    PersonClass::Mafiosi => {
                        match mafiosi_state {
                            MafiosiState::NotAgreeing => {}
                            MafiosiState::NoVotesYet => {
                                mafiosi_state = MafiosiState::VoteFor(a.0[i].0);
                            }
                            MafiosiState::VoteFor(p) => {
                                if p != a.0[i].0 {
                                    mafiosi_state = MafiosiState::NotAgreeing;
                                }
                            }
                        }
                    }
                    PersonClass::Medic => {
                        if medic_vote.is_some() {
                            return Err(QME::BadPersonAssignments);
                        }
                        medic_vote = Some(a.0[i].0);
                    }
                    PersonClass::Necromancer => {
                        if necromancer_vote.is_some() {
                            return Err(QME::BadPersonAssignments);
                        }
                        necromancer_vote = Some(a.0[i].0);
                    }
                    PersonClass::Priest => {
                        if priest_vote.is_some() {
                            return Err(QME::BadPersonAssignments);
                        }
                        priest_vote = Some(a.0[i].0);
                    }
                    _ => {}
                }
            }

            // If the Mafiosi agreed and the medic did not vote for them
            if let MafiosiState::VoteFor(p) = mafiosi_state {
                if medic_vote.map(|m| m != p).unwrap_or(true) {
                    thisgame.people[p.0] = PersonClass::Dead;
                }
            }

            // If the necromancer points at a dead person, make them a zombie
            if let Some(p) = necromancer_vote {
                if let PersonClass::Dead = thisgame.people[p.0] {
                    thisgame.people[p.0] = PersonClass::Zombie;
                }
            }

            // If the priest points at a zombie, make them into a normal dead
            // person
            if let Some(p) = priest_vote {
                if let PersonClass::Zombie = thisgame.people[p.0] {
                    thisgame.people[p.0] = PersonClass::Dead;
                }
            }
        }
        Ok(())
    }

    pub fn do_day(&mut self, a: DayActions) -> Result<(), QME> {
        if a.votes.len() != self.subgames.len() {
            return Err(QME::WrongLengthAction);
        }

        match self.lastnight.take() {
            Some(lastnight) => self.history.push(Turn(lastnight, a.clone())),
            None => return Err(QME::WrongTime),
        }

        // Apply the same rules to each subgame
        for thisgame in self.subgames.iter_mut() {
            let mut zombie_votes = 0;
            let mut votes = 0;
            let mut necromancer_for = false;
            for (i, vote) in a.votes.iter().enumerate() {
                if *vote {
                    use PersonClass as PC;
                    match thisgame.people[i] {
                        PC::Zombie => zombie_votes += 1,
                        PC::Dead => {}
                        _ => votes += 1,
                    }
                    if thisgame.people[i] == PC::Necromancer {
                        necromancer_for = true;
                    }
                }
            }
            if necromancer_for {
                votes += zombie_votes;
            }
            if votes > thisgame.people.len() / 2 {
                thisgame.people[a.accused.0] = PersonClass::Dead;
            }
        }
        Ok(())
    }
}
