use serde::{Deserialize, Serialize};
use reqwest::Error;
use homedir::my_home;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub scene_radius: f32,
    pub azimuth_lines_radius: f32,
    pub azimuth_lines_thickness: f32,
    pub altitude_lines_thickness: f32,
    pub altitude_angle_steps: u32,  // 90/ALTITUDE_ANGLE_STEPS = 10deg per line
    pub azimuth_angle_steps: u32,
    pub star_radius: f32,
    pub user_latitude: f64,
    pub user_longitude: f64,
    pub user_altitude: f64,
    pub max_star_magnitude: f32,
    pub star_magnitude_scale_comp: f32, // make radius adjust with magnitude more tame
    pub shape_resolution: u32,
    pub trail_resolution: usize,
    pub sat_radius: f32,
    pub user_azimuth: f32,
    pub trail_sim_step_seconds: i64,
    pub trail_max_length_seconds: i64,
    pub trail_max_forecast_seconds: i64,
    pub sat_name_font_size: f32,
    pub sat_name_font_width: f32,
    pub tle_fetch_sats: Vec<String>,
    pub altitude_angle_lines_color: String,
    pub azimuth_angle_lines_color: String,
    pub sat_trails_color: String,
    pub sat_color: String,
    pub sat_name_color: String,
    pub sat_name_bg_color: String,
    pub star_color: String,
    pub north_color: String,
    pub tle_update_interval_seconds: i64,
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            scene_radius: 600.0,
            azimuth_lines_radius: 600.0,
            azimuth_lines_thickness: 1.0,
            altitude_lines_thickness: 1.0,
            altitude_angle_steps: 9,
            azimuth_angle_steps: 4,
            star_radius: 2.5,
            user_latitude: 48.8,
            user_longitude: 2.3,
            user_altitude: 0.0,
            max_star_magnitude: 5.0,
            star_magnitude_scale_comp: 3.0,
            shape_resolution: 128,
            trail_resolution: 64,
            sat_radius: 3.0,
            user_azimuth: 0.0,
            trail_sim_step_seconds: 30,
            trail_max_length_seconds: 3600,
            trail_max_forecast_seconds: 3600 * 24,
            sat_name_font_size: 15.0,
            sat_name_font_width: 9.4,
            tle_fetch_sats: vec!["NOAA 15".to_string(), "NOAA 18".to_string(), "NOAA 19".to_string(), "NOAA 20".to_string(), "NOAA 21".to_string(),"METEOR-M 1".to_string(),"METEOR-M 2".to_string(),"METEOR-M2 3".to_string(),"METEOR-M2 4".to_string(),"METOP-A".to_string(),"METOP-B".to_string(),"METOP-C".to_string()],
            altitude_angle_lines_color: "#FFFFFF77".to_string(),
            azimuth_angle_lines_color: "#FFFFFF77".to_string(),
            sat_trails_color: "#FFA500DD".to_string(),
            sat_color: "#FF0000DD".to_string(),
            sat_name_color: "#FFFFFFFF".to_string(),
            sat_name_bg_color: "#000000FF".to_string(),
            star_color: "#FFFFFFDD".to_string(),
            north_color: "#FF0000FF".to_string(),
            tle_update_interval_seconds: 86400*2,
        }
    }
}

pub fn init() -> Config {
    let cfg: Config = match confy::load("ontake/tasogare", "config") {
        Ok(config) => config,
        Err(_) => Config::default(),
    };
    match confy::store("ontake/tasogare", "config", cfg.clone()) {
        Ok(_) => (),
        Err(_) => (), // Doesn't actually matter (this is only for reformatting anyways)
    };
    cfg
}


async fn fetch_tle(sat_name: &str) -> Result<String, Error> {
    println!("Fetching TLE for {}", sat_name);
    let url = format!("https://celestrak.org/NORAD/elements/gp.php?NAME={}&FORMAT=TLE", sat_name);
    let response = reqwest::get(&url).await?;
    let body = response.text().await?;
    Ok(body)
}

pub async fn update_tle(loaded_config: Config) {
    let mut path = my_home().unwrap().expect("couldn't get home directory");
    path.push(".config/ontake/tasogare/TLEDATA");

    if path.as_path().exists() {
        let metadata = std::fs::metadata(path.as_path()).unwrap();
        if (metadata.modified().expect("unable to read TLEDATA metadata").elapsed().expect("unable to get elapsed time since last TLE update").as_secs() as i64) < loaded_config.tle_update_interval_seconds {
            return;
        }
    }

    let mut tle_data = String::new();

    let mut failure = false;

    for sat in &loaded_config.tle_fetch_sats {
        match fetch_tle(sat).await {
            Ok(tle) => tle_data.push_str(&tle),
            Err(_) => failure = true,
        }
    }

    let mut path = my_home().unwrap().expect("couldn't get home directory");
    path.push(".config/ontake/tasogare/TLEDATA");

    if !failure {
        match std::fs::write(&path, tle_data) {
            Ok(_) => (),
            Err(_) => eprintln!("Failed to write TLE data"),
        }
    }
    if !path.as_path().exists() {
        eprintln!("TLEDATA file does not exist.");
    }
}
