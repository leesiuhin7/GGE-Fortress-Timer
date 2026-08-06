use std::time::SystemTime;

use crate::client::session::config::Config;

pub(super) const VERSION_CHECK: &str =
    "<msg t='sys'><body action='verChk' r='0'><ver v='166' /></body></msg>";
pub(super) const AUTO_JOIN: &str = "<msg t='sys'><body action='autoJoin' r='-1'></body></msg>";
pub(super) const ROUND_TRIP: &str = "<msg t='sys'><body action='roundTrip' r='1'></body></msg>";

fn session_id() -> String {
    format!("{:.15e}", rand::random::<f64>() * 2.0_f64.powi(1023)).replace('e', "e+")
}

fn account_id() -> String {
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!(
        "{:.0}{}",
        current_time,
        rand::random_range(100_000..=999_999)
    )
}

pub(super) trait Messages {
    fn server(&self) -> &str;
    fn username(&self) -> &str;
    fn password(&self) -> &str;

    fn ping(&self) -> String {
        format!("%xt%{}%pin%1%<RoundHouseKick>%", self.server())
    }

    fn login(&self) -> String {
        format!(
            "<msg t='sys'><body action='login' r='0'><login z='{}'><nick><![CDATA[]]></nick><pword><![CDATA[undefined%en%0]]></pword></login></body></msg>",
            self.server()
        )
    }

    fn init_round_house_kick(&self) -> String {
        let server = self.server();
        let session_id = session_id();
        format!("%xt%{server}%vck%1%undefined%web-html5%<RoundHouseKick>%{session_id}%")
    }

    fn search_username(&self) -> String {
        let server = self.server();
        let username = self.username();
        format!(r#"%xt%{server}%vln%1%{{"NOM":"{username}"}}%"#)
    }

    fn user_login(&self) -> String {
        let server = self.server();
        let username = self.username();
        let password = self.password();

        let conm = rand::random_range(100..300);
        let rtm = rand::random_range(15..50);
        let account_id = account_id();

        format!(
            r#"%xt%{server}%lli%1%{{"CONM":{conm},"RTM":{rtm},"ID":0,"PL":1,"NOM":"{username}","PW":"{password}","LT":null,"LANG":"en","DID":"0","AID":"{account_id}","KID":"","REF":"https://empire.goodgamestudios.com","GCI":"","SID":9,"PLFID":1}}%"#
        )
    }
}

impl Messages for Config {
    fn server(&self) -> &str {
        &self.server
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn password(&self) -> &str {
        &self.password
    }
}
