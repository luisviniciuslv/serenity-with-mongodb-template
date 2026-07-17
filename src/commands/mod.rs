pub mod adm;
pub mod duelo;
pub mod empresas;
pub mod help_cassino;
pub mod highlow;
pub mod niquel;
pub mod par_ou_impar;
pub mod profile;
pub mod rank;
pub mod rec;

use poise::Command;

use crate::{Data, Error};

pub fn get_commands() -> Vec<Command<Data, Error>> {
    vec![
        profile::profile(),
        rec::rec(),
        par_ou_impar::par_ou_impar(),
        par_ou_impar::par(),
        par_ou_impar::impar(),
        empresas::empresas(),
        niquel::niquel(),
        highlow::highlow(),
        duelo::duelo(),
        adm::add_coins(),
        adm::clear_db(),
        rank::rank(),
        help_cassino::help_cassino(),
    ]
}
