//! Asset-backed diagnostics for the September playtest. No GPU or server.

use korangar_loaders::FileLoader;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::action::ActionsData;
use ragnarok_formats::sprite::SpriteData;
use ragnarok_formats::version::GenericFormatMetadata;

use crate::loaders::GameFileLoader;

fn parse<T: FromBytes>(loader: &GameFileLoader, path: &str) -> Result<T, String> {
    let bytes = loader.get(path).map_err(|error| format!("{path}: {error:?}"))?;
    T::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes)).map_err(|error| format!("{path}: {error:?}"))
}

pub fn run() -> Result<(), String> {
    let loader = GameFileLoader::default();
    loader.load_archives_from_settings();
    let mut invalid = 0;
    for sex in ["남", "여"] {
        let body: ActionsData = parse(&loader, &format!("data\\sprite\\인간족\\몸통\\{sex}\\초보자_{sex}.act"))?;
        for hair in 1..=42 {
            let path = format!("data\\sprite\\인간족\\머리통\\{sex}\\{hair}_{sex}.act");
            let head: ActionsData = parse(&loader, &path)?;
            for facing in 0..8 {
                let body = &body.actions[facing].motions[0];
                let head = &head.actions[facing].motions[0];
                let body_attach = body.attach_points.first().map(|p| p.position);
                let head_attach = head.attach_points.first().map(|p| p.position);
                let delta = body_attach.zip(head_attach).map(|(body, head)| body - head);
                println!(
                    "ATTACH sex={sex} hair={hair} facing={facing} body_count={:?} body={body_attach:?} head_count={:?} \
                     head={head_attach:?} delta={delta:?} clips={:?}",
                    body.attach_point_count,
                    head.attach_point_count,
                    head.sprite_clips
                        .iter()
                        .map(|clip| (clip.sprite_number, clip.position, clip.mirror_on))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    let files = loader.get_files_with_extension(&[".spr", ".act"]);
    let mut headgear = files
        .iter()
        .filter(|path| path.starts_with("data\\sprite\\악세사리\\"))
        .map(|path| path[..path.len() - 4].to_owned())
        .collect::<Vec<_>>();
    headgear.sort();
    headgear.dedup();
    for path in &headgear {
        let result = (|| {
            let sprite: SpriteData = parse(&loader, &format!("{path}.spr"))?;
            let actions: ActionsData = parse(&loader, &format!("{path}.act"))?;
            if actions.actions.is_empty() {
                return Err("no actions".to_owned());
            }
            let mut frames = 0;
            for (action, data) in actions.actions.iter().enumerate() {
                for (motion, frame) in data.motions.iter().enumerate() {
                    frames += 1;
                    for clip in &frame.sprite_clips {
                        if clip.sprite_number == -1 {
                            continue;
                        }
                        let count = if clip.sprite_type == Some(1) {
                            sprite.rgba_image_data.len()
                        } else {
                            sprite.palette_image_data.len()
                        };
                        if clip.sprite_number < 0 || clip.sprite_number as usize >= count {
                            return Err(format!(
                                "action={action} motion={motion} sprite={} type={:?} count={count}",
                                clip.sprite_number, clip.sprite_type
                            ));
                        }
                    }
                }
            }
            Ok((actions.actions.len(), frames))
        })();
        match result {
            Ok((actions, frames)) => println!("HEADGEAR OK {path} actions={actions} motions={frames}"),
            Err(error) => {
                invalid += 1;
                println!("HEADGEAR INVALID {path}: {error}");
            }
        }
    }
    println!("SUMMARY hair_facings=672 headgear_pairs={} invalid={invalid}", headgear.len());
    println!("RENDERING GAP: equipped headgear view IDs are stored but get_entity_part_files does not attach their sprites.");
    if invalid != 0 {
        return Err(format!("{invalid} invalid headgear pairs"));
    }
    Ok(())
}

/// Composed body-vs-head geometry for one facing, printed at every stage.
///
/// The attach-point dump ruled out missing attach points, so whatever moves
/// the head at clock 9 / 12 / 3 happens after the ACT is read: in
/// `apply_child_attach`, `merge_frame`, `finalize_frame_layout`, or the
/// interface's `animation_part_area`. Printing all four stages for the same
/// asset is the only way this has ever been narrowed down; head placement has
/// been guessed wrong five times.
mod composed {
    use std::sync::Arc;

    use cgmath::{Vector2, Zero};
    use korangar_interface::layout::area::Area;
    use ragnarok_formats::action::ActionsData;
    use ragnarok_formats::sprite::SpriteData;
    use ragnarok_packets::ClientTick;

    use crate::loaders::decode_animation_layer_with_sizes;
    use crate::renderer::{animation_part_area, animation_scaling};
    use crate::world::animation::{AnimationState, apply_child_attach, compute_action_layouts, native_layer_motion_index};
    use crate::world::{Actions, AnimationData, AnimationFrame, AnimationLayer, EntityType};

    /// SPR image dimensions in the order `SpriteLoader` uploads them: every
    /// palette image, then every RGBA image. A clip's `sprite_type == 1`
    /// offsets its number by the palette count, so the order is load-bearing.
    fn sprite_sizes(sprite: &SpriteData) -> (usize, Vec<Vector2<i32>>) {
        let palette: Vec<Vector2<i32>> = sprite
            .palette_image_data
            .iter()
            .map(|image| Vector2::new(image.width as i32, image.height as i32))
            .collect();
        let palette_size = palette.len();
        let sizes = palette
            .into_iter()
            .chain(
                sprite
                    .rgba_image_data
                    .iter()
                    .map(|image| Vector2::new(image.width as i32, image.height as i32)),
            )
            .collect();
        (palette_size, sizes)
    }

    pub(super) fn build_actions(data: &ActionsData) -> Arc<Actions> {
        let delays = data.delays.clone().unwrap_or_else(|| data.actions.iter().map(|_| 4.0).collect());
        Arc::new(Actions {
            actions: data.actions.clone(),
            delays,
            events: Vec::new(),
            #[cfg(feature = "debug")]
            actions_data: data.clone(),
        })
    }

    pub(super) fn build_layer(actions: &ActionsData, sprite: &SpriteData, layer_index: usize, path_key: &str) -> AnimationLayer {
        let (palette_size, sizes) = sprite_sizes(sprite);
        decode_animation_layer_with_sizes(
            &build_actions(actions),
            palette_size,
            move |sprite_number| sizes.get(sprite_number).copied().unwrap_or_else(Vector2::zero),
            layer_index,
            Some(path_key.to_owned()),
            None,
        )
    }

    fn describe_parts(frame: &AnimationFrame) -> String {
        frame
            .frame_parts
            .iter()
            .map(|part| {
                format!(
                    "layer={} spr={} offset=({},{}) size=({},{}) mirror={}",
                    part.animation_index, part.sprite_number, part.offset.x, part.offset.y, part.size.x, part.size.y, part.mirror
                )
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn describe_raw_clips(actions: &ActionsData, action_index: usize, motion: usize) -> String {
        let Some(action) = actions.actions.get(action_index) else {
            return "<no action>".to_owned();
        };
        let Some(motion) = action.motions.get(motion) else {
            return "<no motion>".to_owned();
        };
        motion
            .sprite_clips
            .iter()
            .map(|clip| {
                format!(
                    "spr={} type={:?} pos=({},{}) mirror={}",
                    clip.sprite_number, clip.sprite_type, clip.position.x, clip.position.y, clip.mirror_on
                )
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// Print the whole computation for one body + head pair.
    pub(super) fn dump(body_data: &ActionsData, body_sprite: &SpriteData, head_data: &ActionsData, head_sprite: &SpriteData, label: &str) {
        let animation_data = animation(body_data, body_sprite, head_data, head_sprite);
        println!(
            "\nLAYERS {label}: body actions={} head actions={} (head % 8 = {})",
            animation_data.layers[0].animations.len(),
            animation_data.layers[1].animations.len(),
            animation_data.layers[1].animations.len() % 8
        );

        // A square preview at scale 1.0 turns interface pixels into frame
        // pixels, so the printed rectangles are directly comparable to the
        // composed offsets above them.
        let area = Area {
            left: 0.0,
            top: 0.0,
            width: 1000.0,
            height: 1000.0,
        };

        for facing in [0usize, 2, 4, 6] {
            let clock = match facing {
                0 => 6,
                2 => 9,
                4 => 12,
                _ => 3,
            };
            println!("\nCOMPOSED {label} facing={facing} clock={clock}");
            println!("  raw body clips: {}", describe_raw_clips(body_data, facing, 0));
            println!("  raw head clips: {}", describe_raw_clips(head_data, facing, 0));

            let body_frame = &animation_data.layers[0].animations[facing].frames[0];
            let body_attach = body_frame.attach_point;
            println!(
                "  body frame: offset=({},{}) size=({},{}) attach={:?}",
                body_frame.offset.x, body_frame.offset.y, body_frame.size.x, body_frame.size.y, body_attach
            );
            println!("    body parts: {}", describe_parts(body_frame));

            let head_motion = native_layer_motion_index(0, animation_data.layers[1].animations[facing].frames.len());
            let mut head_frame = animation_data.layers[1].animations[facing].frames[head_motion.unwrap_or(0)].clone();
            let head_attach = head_frame.attach_point;
            let delta = body_attach.zip(head_attach).map(|(body, head)| body - head);
            println!(
                "  head frame (pre-attach): offset=({},{}) size=({},{}) attach={:?} delta={:?}",
                head_frame.offset.x, head_frame.offset.y, head_frame.size.x, head_frame.size.y, head_attach, delta
            );
            println!("    head parts: {}", describe_parts(&head_frame));

            apply_child_attach(&mut head_frame, body_attach);
            println!(
                "  head frame (post-attach): offset=({},{}) size=({},{})",
                head_frame.offset.x, head_frame.offset.y, head_frame.size.x, head_frame.size.y
            );
            println!("    head parts: {}", describe_parts(&head_frame));

            let layout = animation_data.action_layouts[facing];
            println!(
                "  action layout: min_top={} max_bottom={} min_left={} max_right={}",
                layout.min_top, layout.max_bottom, layout.min_left, layout.max_right
            );

            let composed = animation_data.compose_idle_frame(facing);
            println!(
                "  composed: offset=({},{}) size=({},{}) parts={}",
                composed.offset.x,
                composed.offset.y,
                composed.size.x,
                composed.size.y,
                composed.frame_parts.len()
            );
            println!("    composed parts: {}", describe_parts(&composed));

            let scaling = animation_scaling(area, &composed, 1.0);
            println!("  interface scaling={scaling}");
            for part in composed.frame_parts.iter() {
                let rect = animation_part_area(area, &composed, part, scaling);
                let name = match part.animation_index {
                    0 => "body",
                    1 => "head",
                    _ => "other",
                };
                println!(
                    "    AREA {name} left={} top={} width={} height={} right={} bottom={} mirror={}",
                    rect.left,
                    rect.top,
                    rect.width,
                    rect.height,
                    rect.left + rect.width,
                    rect.top + rect.height,
                    part.mirror
                );
            }
        }
    }

    /// Composed head-vs-body rectangles for one action group and facing.
    ///
    /// The question a sweep has to answer is not "is the offset pretty" but
    /// "does the head still touch the body". Overlap is the invariant: a head
    /// that has come off its shoulders is disjoint from every body part.
    ///
    /// Returns `(vertical overlap, horizontal overlap, head-vs-body centre
    /// delta)` in frame pixels; a negative overlap is a gap.
    pub(super) fn overlap(animation_data: &AnimationData, action_group: usize, facing: usize) -> Option<(f32, f32, f32)> {
        let animation_state = AnimationState::new(EntityType::Player, ClientTick(0));
        let composed = animation_data.compose_action_motion(&animation_state, action_group * 8 + facing, facing);
        let area = Area {
            left: 0.0,
            top: 0.0,
            width: 1000.0,
            height: 1000.0,
        };
        let scaling = animation_scaling(area, &composed, 1.0);

        let mut body: Option<Area> = None;
        let mut head: Option<Area> = None;
        for part in composed.frame_parts.iter() {
            let rect = animation_part_area(area, &composed, part, scaling);
            let slot = match part.animation_index {
                0 => &mut body,
                1 => &mut head,
                _ => continue,
            };
            *slot = Some(match slot.take() {
                None => rect,
                Some(existing) => {
                    let left = existing.left.min(rect.left);
                    let top = existing.top.min(rect.top);
                    Area {
                        left,
                        top,
                        width: (existing.left + existing.width).max(rect.left + rect.width) - left,
                        height: (existing.top + existing.height).max(rect.top + rect.height) - top,
                    }
                }
            });
        }

        let (body, head) = (body?, head?);
        let vertical = (head.top + head.height).min(body.top + body.height) - head.top.max(body.top);
        let horizontal = (head.left + head.width).min(body.left + body.width) - head.left.max(body.left);
        let centre = (head.left + head.width / 2.0) - (body.left + body.width / 2.0);
        Some((vertical, horizontal, centre))
    }

    /// Assemble a two-layer player animation exactly as the loader would.
    pub(super) fn animation(
        body_data: &ActionsData,
        body_sprite: &SpriteData,
        head_data: &ActionsData,
        head_sprite: &SpriteData,
    ) -> AnimationData {
        let layers = vec![
            build_layer(body_data, body_sprite, 0, "body"),
            build_layer(head_data, head_sprite, 1, "head"),
        ];
        let action_layouts = compute_action_layouts(&layers);
        AnimationData {
            layers,
            delays: build_actions(body_data).delays.clone(),
            action_layouts,
            entity_type: EntityType::Player,
        }
    }
}

/// Print the composed body-vs-head geometry for the male novice with hair 1.
pub fn dump_composed_geometry() -> Result<(), String> {
    let loader = GameFileLoader::default();
    loader.load_archives_from_settings();

    for (sex, label) in [("남", "male novice hair 1"), ("여", "female novice hair 1")] {
        let body_data: ActionsData = parse(&loader, &format!("data\\sprite\\인간족\\몸통\\{sex}\\초보자_{sex}.act"))?;
        let body_sprite: SpriteData = parse(&loader, &format!("data\\sprite\\인간족\\몸통\\{sex}\\초보자_{sex}.spr"))?;
        let head_data: ActionsData = parse(&loader, &format!("data\\sprite\\인간족\\머리통\\{sex}\\1_{sex}.act"))?;
        let head_sprite: SpriteData = parse(&loader, &format!("data\\sprite\\인간족\\머리통\\{sex}\\1_{sex}.spr"))?;
        composed::dump(&body_data, &body_sprite, &head_data, &head_sprite, label);
    }

    Ok(())
}

/// Sweep every hair against every body job, looking for a facing where the
/// composed head no longer touches the composed body.
///
/// The single-asset dump showed male novice hair 1 composing correctly at all
/// eight facings, so if a head really comes off it is asset-specific or
/// action-specific — and only a sweep over the real archive can name which.
/// Idle (group 0) is what an unarmed character stands in; ReadyFight (group 4)
/// is what an armed one stands in, and it is the group whose action index
/// (`32 + facing`) can fall outside a shorter secondary ACT.
pub fn sweep_composed_geometry() -> Result<(), String> {
    let loader = GameFileLoader::default();
    loader.load_archives_from_settings();

    let body_root = "data\\sprite\\인간족\\몸통";
    let jobs: Vec<String> = {
        let mut jobs: Vec<String> = loader
            .get_files_with_extension(&[".act"])
            .iter()
            .filter_map(|path| path.strip_prefix(&format!("{body_root}\\남\\")))
            .filter_map(|name| name.strip_suffix("_남.act"))
            .map(|name| name.to_owned())
            .collect();
        jobs.sort();
        jobs.dedup();
        jobs
    };
    println!("SWEEP jobs={}", jobs.len());

    let mut detached = 0;
    let mut checked = 0;
    let mut tightest: Vec<(i64, String)> = Vec::new();

    for sex in ["남", "여"] {
        for job in &jobs {
            let body_path = format!("{body_root}\\{sex}\\{job}_{sex}");
            let (Ok(body_data), Ok(body_sprite)) = (
                parse::<ActionsData>(&loader, &format!("{body_path}.act")),
                parse::<SpriteData>(&loader, &format!("{body_path}.spr")),
            ) else {
                continue;
            };

            for hair in 1..=42 {
                let head_path = format!("data\\sprite\\인간족\\머리통\\{sex}\\{hair}_{sex}");
                let (Ok(head_data), Ok(head_sprite)) = (
                    parse::<ActionsData>(&loader, &format!("{head_path}.act")),
                    parse::<SpriteData>(&loader, &format!("{head_path}.spr")),
                ) else {
                    continue;
                };

                let animation_data = composed::animation(&body_data, &body_sprite, &head_data, &head_sprite);
                let groups = animation_data.layers[0].animations.len() / 8;

                for action_group in 0..groups {
                    for facing in 0..8 {
                        let Some((vertical, horizontal, centre)) = composed::overlap(&animation_data, action_group, facing) else {
                            continue;
                        };
                        checked += 1;
                        tightest.push((
                            (vertical.min(horizontal) * 100.0) as i64,
                            format!(
                                "{sex} {job} hair={hair} group={action_group} facing={facing} v={vertical} h={horizontal} centre={centre}"
                            ),
                        ));
                        if vertical <= 0.0 || horizontal <= 0.0 {
                            detached += 1;
                            if detached <= 40 {
                                println!(
                                    "DETACHED {sex} {job} hair={hair} group={action_group} facing={facing} vertical={vertical} \
                                     horizontal={horizontal} centre={centre}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    tightest.sort_by_key(|(score, _)| *score);
    println!("\nTIGHTEST 20:");
    for (_, line) in tightest.iter().take(20) {
        println!("  {line}");
    }
    println!("\nSWEEP checked={checked} detached={detached}");
    Ok(())
}

/// One optional attach point per facing, as authored in the ACT.
type AttachByFacing = Vec<Option<(i32, i32)>>;

/// Attach-point coverage for every player body and head ACT in the archive.
///
/// The composed-geometry sweep showed head and body are locked together by
/// construction: `apply_child_attach` translates every head part by
/// `body_attach - head_attach`, so they cannot drift apart — *unless* that
/// call returns early, which it does when either side has no attach point.
/// That early return is the only path in the compositor that can leave a head
/// unparented, so this is the check that decides whether the 9/12/3 report is
/// an asset problem or a code problem.
///
/// It also counts the case the removed `attach_point_count == Some(1)` gate
/// used to drop: a motion that authors points while declaring a count other
/// than one.
pub fn sweep_attach_points() -> Result<(), String> {
    let loader = GameFileLoader::default();
    loader.load_archives_from_settings();

    let body_root = "data\\sprite\\인간족\\몸통";
    let head_root = "data\\sprite\\인간족\\머리통";
    let mut paths: Vec<String> = loader
        .get_files_with_extension(&[".act"])
        .iter()
        .filter(|path| path.starts_with(body_root) || path.starts_with(head_root))
        .map(|path| path.to_owned())
        .collect();
    paths.sort();
    paths.dedup();

    let mut standing: std::collections::BTreeMap<String, Vec<(usize, usize)>> = std::collections::BTreeMap::new();
    let mut motions = 0u64;
    let mut missing = 0u64;
    let mut count_not_one = 0u64;
    let mut reported = 0;

    for path in &paths {
        let Ok(actions) = parse::<ActionsData>(&loader, path) else {
            continue;
        };
        for (action_index, action) in actions.actions.iter().enumerate() {
            for (motion_index, motion) in action.motions.iter().enumerate() {
                // A motion that draws nothing needs no attach point; the
                // compositor never emits a part for it either.
                if motion.sprite_clips.iter().all(|clip| clip.sprite_number == -1) {
                    continue;
                }
                motions += 1;
                let authored = motion.attach_points.len();
                let declared = motion.attach_point_count;
                if authored == 0 {
                    missing += 1;
                    // Idle (0) and ReadyFight (4) are the standing groups a
                    // character is in on select and while idle in the field —
                    // the ones the 9/12/3 report is about.
                    if matches!(action_index / 8, 0 | 4) {
                        let facings = standing.entry(path.clone()).or_default();
                        let facing = action_index % 8;
                        if !facings.contains(&(action_index / 8, facing)) {
                            facings.push((action_index / 8, facing));
                        }
                    }
                    let _ = &mut reported;
                } else if declared != Some(1) {
                    count_not_one += 1;
                    if reported < 30 {
                        reported += 1;
                        println!(
                            "COUNT != 1 {path} action={action_index} (group={} facing={}) motion={motion_index} declared={declared:?} \
                             authored={authored}",
                            action_index / 8,
                            action_index % 8
                        );
                    }
                }
            }
        }
    }

    println!("\nSTANDING GROUPS WITH NO ATTACH POINT ({} files):", standing.len());
    for (path, mut facings) in standing {
        facings.sort();
        println!("  {path} -> {facings:?}");
    }

    println!(
        "\nATTACH acts={} drawing_motions={motions} missing={missing} count_not_one={count_not_one}",
        paths.len()
    );
    Ok(())
}

/// Confirm which Lua tables the archive ships for headgear view IDs.
pub fn list_accessory_tables() -> Result<(), String> {
    let loader = GameFileLoader::default();
    loader.load_archives_from_settings();
    for path in loader.get_files_with_extension(&[".lub", ".lua"]) {
        let lowered = path.to_lowercase();
        if lowered.contains("acc") || lowered.contains("headgear") {
            println!("LUA {path}");
        }
    }
    Ok(())
}

/// Compare headgear attach points against head attach points.
///
/// Everything about where a hat belongs follows from this: if a headgear ACT
/// authors the same attach point as the head does for the same facing, then
/// the hat is in the head's own coordinate frame and the classic
/// `-child + parent` rule with the **body** attach lands it correctly. If they
/// differ, the hat is parented to the head and needs the head's attach.
pub fn compare_headgear_attach() -> Result<(), String> {
    let loader = GameFileLoader::default();
    loader.load_archives_from_settings();

    for sex in ["남", "여"] {
        // Do the heads even agree with each other?
        let mut head_points: Vec<(usize, AttachByFacing)> = Vec::new();
        for hair in 1..=42 {
            let Ok(head) = parse::<ActionsData>(&loader, &format!("data\\sprite\\인간족\\머리통\\{sex}\\{hair}_{sex}.act")) else {
                continue;
            };
            let points = (0..8)
                .map(|facing| {
                    head.actions
                        .get(facing)
                        .and_then(|action| action.motions.first())
                        .and_then(|motion| motion.attach_points.first())
                        .map(|point| (point.position.x, point.position.y))
                })
                .collect();
            head_points.push((hair, points));
        }
        let reference = head_points[0].1.clone();
        let disagreeing: Vec<usize> = head_points
            .iter()
            .filter(|(_, points)| *points != reference)
            .map(|(hair, _)| *hair)
            .collect();
        println!("HEADS {sex}: idle attach {reference:?}; hairs disagreeing with hair 1: {disagreeing:?}");

        let mut hats: Vec<String> = loader
            .get_files_with_extension(&[".act"])
            .iter()
            .filter(|path| path.starts_with(&format!("data\\sprite\\악세사리\\{sex}\\")))
            .map(|path| path.to_owned())
            .collect();
        hats.sort();

        let mut drawing_without_attach = 0u64;
        let mut unparented: Vec<String> = Vec::new();
        let mut same = 0;
        let mut differ = 0;
        let mut missing = 0;
        let mut examples = Vec::new();
        for path in &hats {
            let Ok(hat) = parse::<ActionsData>(&loader, path) else {
                continue;
            };
            let points: AttachByFacing = (0..8)
                .map(|facing| {
                    hat.actions
                        .get(facing)
                        .and_then(|action| action.motions.first())
                        .and_then(|motion| motion.attach_points.first())
                        .map(|point| (point.position.x, point.position.y))
                })
                .collect();
            // A motion that draws nothing needs no attach point. One that
            // draws *and* has none is the real hazard: `apply_child_attach`
            // returns early and the hat lands at its raw ACT position.
            for (action_index, action) in hat.actions.iter().enumerate() {
                for (motion_index, motion) in action.motions.iter().enumerate() {
                    if motion.sprite_clips.iter().all(|clip| clip.sprite_number == -1) {
                        continue;
                    }
                    if motion.attach_points.is_empty() {
                        drawing_without_attach += 1;
                        if unparented.len() < 8 {
                            unparented.push(format!("{path} action={action_index} motion={motion_index}"));
                        }
                    }
                }
            }
            if points.iter().all(|point| point.is_none()) {
                missing += 1;
            } else if points == reference {
                same += 1;
            } else {
                differ += 1;
                if examples.len() < 5 {
                    examples.push(format!("{path} -> {points:?}"));
                }
            }
        }
        println!(
            "HATS {sex}: total={} same_as_head={same} differ={differ} no_attach={missing} \
             drawing_motions_without_attach={drawing_without_attach}",
            hats.len()
        );
        for line in &unparented {
            println!("  UNPARENTED {line}");
        }
        for example in examples {
            println!("  DIFFERS {example}");
        }
    }
    Ok(())
}

/// Resolve every headgear view id through the library and check the sprite
/// pair actually exists.
///
/// Nine of eighteen skill-name guesses on this project were wrong; a view-id
/// table is exactly the kind of mapping that looks right and resolves to
/// nothing. This is the mechanical check that it does not.
pub fn verify_headgear_lookup() -> Result<(), String> {
    let loader = GameFileLoader::default();
    loader.load_archives_from_settings();
    // Load just this table: building the whole `Library` drags in unrelated
    // lua tables whose failures would hide the answer.
    let table =
        <crate::world::AccessoryName as crate::world::Table>::load(&loader).map_err(|error| format!("accessory table: {error:?}"))?;
    println!("HEADGEAR LOOKUP entries={}", table.len());

    for (female, sex_token) in [(false, "남"), (true, "여")] {
        let mut resolved = 0;
        let mut missing_sprite = Vec::new();
        let mut unmapped = 0;

        for view_id in 1..=2000u16 {
            let Some(name) = table.get(&crate::world::AccessoryNameKey { view_id, female }) else {
                unmapped += 1;
                continue;
            };
            let name = name.as_str();
            if name.is_empty() {
                unmapped += 1;
                continue;
            }
            let part = match name.starts_with('_') {
                true => format!("data\\sprite\\악세사리\\{sex_token}\\{sex_token}{name}"),
                false => format!("data\\sprite\\악세사리\\{sex_token}\\{sex_token}_{name}"),
            };
            if loader.file_exists(&format!("{part}.spr").to_lowercase()) && loader.file_exists(&format!("{part}.act").to_lowercase()) {
                resolved += 1;
            } else {
                missing_sprite.push(format!("{view_id} -> {name}"));
            }
        }

        println!(
            "HEADGEAR LOOKUP {sex_token}: resolved={resolved} missing_sprite={} unmapped_ids={unmapped}",
            missing_sprite.len()
        );
        for line in missing_sprite.iter().take(10) {
            println!("  MISSING {line}");
        }
    }
    Ok(())
}
