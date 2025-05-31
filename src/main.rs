mod config;

use clap::Parser;
use satkit::types::Vector3;
use std::f32::consts::PI;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

use star_catalog::{hipparcos, Catalog};
use homedir::my_home;

/// TODO: https://docs.rs/star-catalog/latest/star_catalog/struct.Star.html
/// https://docs.rs/map_3d/latest/map_3d/fn.eci2aer.html
///
///
/// Also add satellites and constellations

/// With https://docs.rs/sgp4/latest/sgp4/

#[derive(Parser, Debug)]
#[clap(author = "Louis Dalibard (OnTake/make-42)", version, about)]
/// Application configuration
struct Args {
    /// whether to be verbose
    #[arg(short = 'v')]
    verbose: bool,

    /// an optional name to greet
    #[arg()]
    name: Option<String>,
}

use bevy::{math::vec2, prelude::*, render::mesh::AnnulusMeshBuilder, sprite::Anchor};

#[tokio::main]
async fn main() {
    let _args = Args::parse();
    let loaded_config = config::init();
    config::update_tle(loaded_config).await;
    App::new()
        .add_plugins(
            DefaultPlugins/*.set(ImagePlugin::default_nearest())*/.set(WindowPlugin {
                primary_window: Some(Window {
                    // Setting `transparent` allows the `ClearColor`'s alpha value to take effect
                    transparent: true,
                    // Disabling window decorations to make it feel more like a widget than a window
                    decorations: false,
                    // present_mode: PresentMode::Immediate,
                    #[cfg(target_os = "macos")]
                    composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                    ..default()
                }),
                ..default()
            }),
        )
        // ClearColor must have 0 alpha, otherwise some color will bleed through
        .insert_resource(ClearColor(Color::NONE))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                set_star_positions,
                set_sat_positions,
                compute_sat_trails,
                compute_sat_positions,
                draw_satellite_trail,
            ),
        )
        .run();
}

/// A marker component for our shapes so we can query them separately
#[derive(Component)]
struct Shape;

/// A marker component for our stars so we can query them separately
#[derive(Component)]
struct Star {
    pub id: usize,
    pub ra: f64,
    pub de: f64,
    pub ly: f32,
    pub mag: f32,
    pub bv: f32,
    pub vector: Vec3,
    pub loaded_config: config::Config,
}

#[derive(Component)]
struct Satellite {
    pub name: String,
    pub constants: sgp4::Constants,
    pub elements: sgp4::Elements,
    pub last_pass_end_datetime: DateTime<Utc>,
    pub positions: Vec<Vec2>,
    pub times: Vec<i64>,
    pub loaded_config: config::Config,
}

#[derive(Component)]
struct SatelliteTrail {
    pub name: String,
    pub constants: sgp4::Constants,
    pub elements: sgp4::Elements,
    pub last_pass_end_datetime: DateTime<Utc>,
    pub spline:  CubicCardinalSpline<Vec2>,
    pub loaded_config: config::Config,
    pub color: Color,
}

fn hexstr2color(hex_color: &String) -> Color {
    return bevy::prelude::Color::Srgba(Srgba::hex(hex_color).unwrap());
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let loaded_config = config::init();
    let font = asset_server.load("fonts/FiraMono-Bold.ttf");
    let altitude_angle_lines_material = materials.add(hexstr2color(&loaded_config.altitude_angle_lines_color));
    let azimuth_angle_lines_material = materials.add(hexstr2color(&loaded_config.azimuth_angle_lines_color));
    let sat_trails_color = hexstr2color(&loaded_config.sat_trails_color);
    let sat_material = materials.add(hexstr2color(&loaded_config.sat_color));
    let sat_name_color =hexstr2color(&loaded_config.sat_name_color);
    let sat_name_bg_color = hexstr2color(&loaded_config.sat_name_bg_color);
    let star_material = materials.add(hexstr2color(&loaded_config.star_color));
    let north_color = hexstr2color(&loaded_config.north_color);

    for m in 0..(loaded_config.altitude_angle_steps) {
        let angle = m as f32/(loaded_config.altitude_angle_steps) as f32*PI/2.0;
        commands.spawn((
            Mesh2d(meshes.add(AnnulusMeshBuilder {annulus:Annulus{inner_circle:Circle{radius:loaded_config.scene_radius*angle.cos()-loaded_config.altitude_lines_thickness/2.0 },
                outer_circle:Circle{radius:loaded_config.scene_radius*angle.cos()+loaded_config.altitude_lines_thickness/2.0}}, resolution: loaded_config.shape_resolution})),
            MeshMaterial2d(altitude_angle_lines_material.clone()),
            Transform::from_xyz(
                0.,
                0.,
                -20.,
            ),
            Shape,
        ));
    };

    for m in 0..(loaded_config.azimuth_angle_steps) {
        let angle = m as f32/(loaded_config.azimuth_angle_steps) as f32*PI;
        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(loaded_config.azimuth_lines_thickness, loaded_config.azimuth_lines_radius*2.))),
            MeshMaterial2d(azimuth_angle_lines_material.clone()),
            Transform::from_xyz(
                0.,
                0.,
                -30.,
            ).with_rotation(Quat::from_rotation_z(angle+loaded_config.user_azimuth / 180.0 * std::f32::consts::PI)),
            Shape,
        ));
    };

    let s = std::fs::read_to_string("assets/data/hipparcos.json").expect("couldn't read hipparcos.json");
    let mut catalog: Catalog = serde_json::from_str(&s).expect("couldn't parse hipparcos.json");
    catalog.sort();
    catalog.add_names(hipparcos::HIP_ALIASES, true).unwrap();
    catalog.derive_data();

    catalog.iter_stars().for_each(|star| {
        if star.mag < loaded_config.max_star_magnitude{
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(loaded_config.star_radius/(star.mag+loaded_config.star_magnitude_scale_comp)*loaded_config.star_magnitude_scale_comp))),
            MeshMaterial2d(star_material.clone()),
            Transform::from_xyz(
                0.,
                0.,
                0.,
            ),
            Star {
                id: star.id,
                ra: star.ra,
                de: star.de,
                ly: star.ly,
                mag: star.mag,
                bv: star.bv,
                vector: Vec3::new(star.vector[0] as f32, star.vector[1] as f32, star.vector[2] as f32),
                loaded_config: loaded_config.clone(),
            },
        ));
        };
    });
    let mut path = my_home().unwrap().expect("couldn't get home directory");
    path.push(".config/ontake/tasogare/TLEDATA");
    let tle_content = std::fs::read_to_string(path.as_path()).expect("couldn't read input TLE");
    let tle_lines: Vec<&str> = tle_content.lines().collect();
    let text_font = TextFont {
        font: font.clone(),
        font_size: loaded_config.sat_name_font_size,
        ..Default::default()
    };
    for i in (0..tle_lines.len()).step_by(3) {
        let name = tle_lines[i].to_owned().trim().to_string();
        let line1 = tle_lines[i + 1];
        let line2 = tle_lines[i + 2];
    let elements = sgp4::Elements::from_tle(
           Some(name),
           line1.as_bytes(),
           line2.as_bytes(),
       ).unwrap();

    let constants = sgp4::Constants::from_elements_afspc_compatibility_mode(&elements).unwrap();




    commands.spawn((
        Mesh2d(meshes.add(Circle::new(loaded_config.sat_radius))),
        MeshMaterial2d(sat_material.clone()),
        Transform::from_xyz(
            0.,
            0.,
            0.,
        ),
        Satellite {
            name:elements.object_name.clone().unwrap(),
            constants: constants.clone(),
            elements: elements.clone(),
            times: Vec::new(),
            positions: Vec::new(),
            last_pass_end_datetime: Utc::now(),
            loaded_config: loaded_config.clone(),
        },
    )).with_children(|commands| {
        commands.spawn((
            (
                Text2d::new(elements.object_name.clone().unwrap()),
                text_font.clone(),
                TextLayout::new(JustifyText::Left, LineBreak::AnyCharacter),
                TextColor(sat_name_color),
                Anchor::TopLeft,
                //Transform::from_translation(Vec3::Z),
            ),
        ));
        let bg_width = loaded_config.sat_name_font_width*(elements.object_name.clone().unwrap().len() as f32);
        commands
                .spawn((
                    Sprite {
                        color: sat_name_bg_color,
                        custom_size: Some(Vec2::new(bg_width.clone(), loaded_config.sat_name_font_size)),
                        ..Default::default()
                    },
                    Transform::from_translation(-10. * Vec3::Z-loaded_config.sat_name_font_size/2.*Vec3::Y+bg_width/2.*Vec3::X-loaded_config.sat_name_font_size/10.*Vec3::X),
                ));
    });

    // Add satellite trail here
    commands.spawn(SatelliteTrail{
        name:elements.object_name.clone().unwrap(),
        constants: constants,
        elements: elements,
        last_pass_end_datetime: chrono::Utc::now(),
        spline: CubicCardinalSpline::new(0.5,Vec::new()),
        loaded_config: loaded_config.clone(),
        color: sat_trails_color,
    });
    };
    commands.spawn(
        (
            Text2d::new("N"),
            text_font,
            TextLayout::new(JustifyText::Left, LineBreak::AnyCharacter),
            TextColor(north_color),
            Anchor::BottomCenter,
            Transform::from_xyz(
                -loaded_config.scene_radius*(loaded_config.user_azimuth/180.*PI).sin(),
                loaded_config.scene_radius*(loaded_config.user_azimuth/180.*PI).cos(),
                -60.,
            ).with_rotation(Quat::from_rotation_z(loaded_config.user_azimuth/180.*PI),
        ),
    ));

    /*commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));*/



    commands.spawn((
        Camera2d::default(),Msaa::Sample4
    ));
}

/// Return the round toward zero value of the input
pub fn fix(x : f64) -> f64 {
    let mut out = x;
    if out<0.0 {
        out = x.ceil();
    } else {
        out = x.floor();
    }
    out
}
fn gst() -> f64{
    let current_date = chrono::Utc::now();
    gst_from_datetime(current_date)
}

fn gst_from_datetime(date: DateTime<Utc>) -> f64{
    let mut year = date.year() as f64;
    let mut month = date.month() as f64;
    let day = date.day() as f64;
    let h = date.hour() as f64;
    let m = date.minute() as f64;
    let s = date.second() as f64+(date.timestamp_millis().rem_euclid(1000) as f64)/1000.;
    if month<3.0 {
        year = year - 1.0;
        month = month + 12.0;
    }
    let a = fix(year/100.0);
    let b = 2.0 - a + fix(a/4.0);
    let c = ((s/60.0 + m)/60.0 + h)/24.0;
    let jd = fix(365.25 * (year + 4716.0)) + fix(30.6001*(month + 1.0)) + day + b - 1524.5 + c;
    let t_ut1 = (jd - 2451545.0)/36525.0;
    let gmst_sec = 67310.54841 + 3.164400184812866e+09 * t_ut1 + 0.093104 * t_ut1 * t_ut1
                        - 6.2e-6 * t_ut1 * t_ut1 * t_ut1;
    return (gmst_sec * 2.0 * std::f64::consts::PI / 86400.0).rem_euclid(2.0 * std::f64::consts::PI)
}

fn set_star_positions(mut query: Query<(&mut Transform,&Star), With<Star>>, _: Res<Time>) {
    let gst = gst();
    for (mut transform,star) in &mut query {
        let nr = map_3d::EARTH_RADIUS*1000.;  // Should reduce errors due to them not being at infinity
        let rx = star.vector.x as f64*nr;
        let ry = star.vector.y as f64*nr;
        let rz = star.vector.z as f64*nr;

        let (mut az,el,_) = map_3d::eci2aer(gst,rx,ry,rz,(star.loaded_config.user_latitude as f64)/180.0*(PI as f64),(star.loaded_config.user_longitude as f64)/180.0*(PI as f64),star.loaded_config.user_altitude as f64,map_3d::Ellipsoid::WGS84);
        az -= (star.loaded_config.user_azimuth / 180.0 * std::f32::consts::PI) as f64;
        transform.translation = transform.local_x()*((star.loaded_config.scene_radius as f64*az.sin()*el.cos()) as f32)+transform.local_y()*((star.loaded_config.scene_radius as f64*az.cos()*el.cos()) as f32)-40.0*transform.local_z();
        if el<0.0{
            transform.translation = transform.local_x()*100000000000000000000000.0;
        }
    }
}

fn set_sat_positions(mut query: Query<(&mut Transform,&Satellite), With<Satellite>>, _: Res<Time>) {
    let current_date = chrono::Utc::now();
    for (mut transform,sat) in &mut query {
        if sat.times.len() == 0 {
            transform.translation = transform.local_x()*100000000000000000000000.0;
            continue;
        }
        if sat.times[0] <= current_date.timestamp_millis() && sat.times[sat.times.len()-1] >= current_date.timestamp_millis() {
            let mut left = 0;
            let mut right = sat.times.len() - 1;
            while left < right {
                let mid = left + (right - left) / 2;
                if sat.times[mid] < current_date.timestamp_millis() {
                    left = mid + 1;
                } else {
                    right = mid;
                }
            }
            let i = left;
            let mut lambda = 0.0;
            let mut a = i;
            let mut b = i;
            if i != 0 {
                if i >= sat.times.len() {
                    transform.translation = transform.local_x()*100000000000000000000000.0;
                    continue;
                }
                a = i-1;
                b = i;
                lambda = ((current_date.timestamp_millis()-sat.times[a]) as f32)/((sat.times[b]-sat.times[a]) as f32);
            }
            let interp_x = sat.positions[a][0]*(1.0-lambda)+sat.positions[b][0]*lambda;
            let interp_y = sat.positions[a][1]*(1.0-lambda)+sat.positions[b][1]*lambda;
            transform.translation = transform.local_x()*(interp_x as f32)+transform.local_y()*(interp_y as f32)+transform.local_z()*0.2;
        } else {
            transform.translation = transform.local_x()*100000000000000000000000.0;
        }
    }
}

fn compute_sat_positions(mut query: Query<&mut Satellite, With<Satellite>>, _: Res<Time>) {
    for mut sat in &mut query {
    let mut current_date = chrono::Utc::now();
    let mut forecasted_aos_datetime = chrono::Utc::now();
        if current_date.signed_duration_since(sat.last_pass_end_datetime) > Duration::seconds(0 as i64) {
            println!("Computing positions for satellite {} by propagating keplerian elements", sat.name);
            let mut points = Vec::new();
            let mut times = Vec::new();
            let mut passed_over_horizon = false;
            let mut passed_under_horizon = false;
            while current_date.signed_duration_since(sat.last_pass_end_datetime) < Duration::seconds(sat.loaded_config.trail_max_forecast_seconds) && current_date.signed_duration_since(forecasted_aos_datetime) < Duration::seconds(sat.loaded_config.trail_max_length_seconds) && !(passed_over_horizon && passed_under_horizon) {
            let gst = gst_from_datetime(current_date);
            let prediction = sat.constants.propagate_afspc_compatibility_mode(sat.elements.datetime_to_minutes_since_epoch(&current_date.naive_utc()).unwrap()).unwrap();
            let rx = prediction.position[0]*1000.0 as f64;
            let ry = prediction.position[1]*1000.0 as f64;
            let rz = prediction.position[2]*1000.0 as f64;
            let q = satkit::frametransform::qteme2gcrf(&satkit::Instant::new(current_date.timestamp_micros()));
            let roted_vect = q.transform_vector(&Vector3::new(rx, ry, rz));

            let (mut az,el,_) = map_3d::eci2aer(gst,roted_vect[0],roted_vect[1],roted_vect[2],(sat.loaded_config.user_latitude as f64)/180.0*(PI as f64),(sat.loaded_config.user_longitude as f64)/180.0*(PI as f64),sat.loaded_config.user_altitude as f64,map_3d::Ellipsoid::WGS84);
            //println!("Azimuth: {}, Elevation: {}", az, el);
            az -= (sat.loaded_config.user_azimuth / 180.0 * std::f32::consts::PI) as f64;
            if !passed_over_horizon{
                forecasted_aos_datetime = current_date;
            }
            if !passed_over_horizon && el>0.0{
                passed_over_horizon = true;
            }
            if passed_over_horizon && el<0.0{
                passed_under_horizon = true;
            }
            if passed_over_horizon{
                points.push(vec2((sat.loaded_config.scene_radius as f64*az.sin()*el.cos()) as f32, (sat.loaded_config.scene_radius as f64*az.cos()*el.cos()) as f32));
                times.push(current_date.timestamp_millis());
            }
           current_date = current_date.checked_add_signed(Duration::seconds(sat.loaded_config.trail_sim_step_seconds)).unwrap();
        }
        sat.positions = points;
        sat.times = times;
        sat.last_pass_end_datetime = current_date;
        }
    }
}

fn compute_sat_trails(mut query: Query<&mut SatelliteTrail, With<SatelliteTrail>>, _: Res<Time>) {
    for mut sat in &mut query {
    let mut current_date = chrono::Utc::now();
    let mut forecasted_aos_datetime = chrono::Utc::now();
        if current_date.signed_duration_since(sat.last_pass_end_datetime) > Duration::seconds(0 as i64) {
            println!("Computing trails for satellite {} by propagating keplerian elements", sat.name);
            let mut points = Vec::new();
            let mut passed_over_horizon = false;
            let mut passed_under_horizon = false;
            while current_date.signed_duration_since(sat.last_pass_end_datetime) < Duration::seconds(sat.loaded_config.trail_max_forecast_seconds) && current_date.signed_duration_since(forecasted_aos_datetime) < Duration::seconds(sat.loaded_config.trail_max_length_seconds) && !(passed_over_horizon && passed_under_horizon) {
            let gst = gst_from_datetime(current_date);
            let prediction = sat.constants.propagate_afspc_compatibility_mode(sat.elements.datetime_to_minutes_since_epoch(&current_date.naive_utc()).unwrap()).unwrap();
            let rx = prediction.position[0]*1000.0 as f64;
            let ry = prediction.position[1]*1000.0 as f64;
            let rz = prediction.position[2]*1000.0 as f64;
            let q = satkit::frametransform::qteme2gcrf(&satkit::Instant::new(current_date.timestamp_micros()));
            let roted_vect = q.transform_vector(&Vector3::new(rx, ry, rz));

            let (mut az,el,_) = map_3d::eci2aer(gst,roted_vect[0],roted_vect[1],roted_vect[2],(sat.loaded_config.user_latitude as f64)/180.0*(PI as f64),(sat.loaded_config.user_longitude as f64)/180.0*(PI as f64),sat.loaded_config.user_altitude as f64,map_3d::Ellipsoid::WGS84);
            //println!("Azimuth: {}, Elevation: {}", az, el);
            az -= (sat.loaded_config.user_azimuth / 180.0 * std::f32::consts::PI) as f64;
            if !passed_over_horizon{
                forecasted_aos_datetime = current_date;
            }
            if !passed_over_horizon && el>0.0{
                passed_over_horizon = true;
            }
            if passed_over_horizon && el<0.0{
                passed_under_horizon = true;
            }
            if passed_over_horizon{
                points.push(vec2((sat.loaded_config.scene_radius as f64*az.sin()*el.cos()) as f32, (sat.loaded_config.scene_radius as f64*az.cos()*el.cos()) as f32));
            }
            current_date = current_date.checked_add_signed(Duration::seconds(sat.loaded_config.trail_sim_step_seconds)).unwrap();
        }
        sat.spline = CubicCardinalSpline::new(0.5,points);
        sat.last_pass_end_datetime = current_date;
        }
    }
}

fn draw_satellite_trail(query: Query<&SatelliteTrail, With<SatelliteTrail>>, _: Res<Time>, mut gizmos: Gizmos) {
        for trail in &query {
            let spline = trail.spline.clone();
            match spline.to_curve() {
                Ok(curve) => {
                    gizmos.linestrip(
                        curve.iter_positions(trail.loaded_config.trail_resolution).map(|pt| pt.extend(0.0)),
                        trail.color,
                    );
                }
                Err(_) => continue,
            }
        }
}
