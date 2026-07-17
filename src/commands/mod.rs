pub mod par_ou_impar;
pub mod profile;
pub mod adm;
pub mod rec;
pub mod empresas;
pub mod niquel;
pub mod duelo;
pub mod highlow;
pub mod rank;
pub mod help_cassino;

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
