use distant_horizons_for_horizon_client::{
    BlockInfo, BlockTable, RenderSettings, RenderSource, SampledRun, FullDataSource,
    build_quads, column_from_runs, gen_step, material, pos, shade_open_water, Dir,
};

fn blocks() -> BlockTable {
    BlockTable { states: vec![
        BlockInfo { air:true, material:material::AIR, ..BlockInfo::default() },
        BlockInfo { solid:true, opacity:16, alpha:255, material:material::STONE, ..BlockInfo::default() },
        BlockInfo { liquid:true, opacity:1, alpha:160, material:material::WATER, ..BlockInfo::default() },
        // A different water-like state (e.g. seagrass) must not interrupt depth.
        BlockInfo { liquid:true, opacity:1, alpha:160, material:material::WATER, ..BlockInfo::default() },
        BlockInfo { liquid:true, opacity:1, alpha:255, material:material::LAVA, ..BlockInfo::default() },
    ] }
}

fn run(min:i32, max:i32, state:u32, sky:u8) -> SampledRun {
    SampledRun { min,max,state,sky,biome:1,block:7 }
}

#[test]
fn saved_and_generated_seabed_faces_have_identical_light() {
    let blocks = blocks();
    for depth in [1,2,4,8,14,15,16,32] {
        let floor = 63-depth;
        let mut cached = vec![run(-64, floor, 1, 15),run(floor,63,2,15)];
        let mut generated = vec![run(-64,floor,1,(15-depth).max(0) as u8)];
        for y in floor..63 {
            generated.push(run(y,y+1,2,(15-(62-y)).max(0) as u8));
        }
        shade_open_water(&mut cached, &blocks);
        shade_open_water(&mut generated, &blocks);
        assert_eq!(cached[0], generated[0]);
        assert_eq!(cached[0].sky,(15-depth).max(0) as u8);
        assert!(cached.iter().all(|r|r.block==7));
        let mesh = |runs:&[SampledRun]| {
            let mut source=FullDataSource::empty(pos::encode(6,0,0));
            let column=column_from_runs(&mut source.mapping,runs,-64,320);
            source.set_column(0,0,column,gen_step::FEATURES);
            let settings=RenderSettings {
                cave_culling:true,cave_culling_height:60,min_y:-64,
            };
            let rendered=RenderSource::from_full(&source,&blocks,&settings);
            build_quads(&rendered, [None,None,None,None], &settings, |_,_|false)
        };
        for quads in [mesh(&cached),mesh(&generated)] {
            let face=quads.iter().find(|q|q.dir==Dir::Up && q.plane==floor).unwrap();
            assert_eq!(face.sky,(15-depth).max(0) as u8,"depth {depth}");
            assert_eq!(face.block,7,"preserve emitted light");
        }
    }
}

#[test]
fn submerged_water_like_states_count_towards_depth() {
    let mut runs=vec![run(-64,53,1,15),run(53,57,3,15),run(57,63,2,15)];
    shade_open_water(&mut runs,&blocks());
    assert_eq!(runs.iter().map(|r|r.sky).collect::<Vec<_>>(),vec![5,9,15]);
    let once=runs.clone();
    shade_open_water(&mut runs,&blocks());
    assert_eq!(runs,once,"normalization is idempotent");
}

#[test]
fn covered_water_lava_dry_land_and_air_gaps_keep_their_light() {
    for mut runs in [
        vec![run(-64,53,1,3),run(53,63,2,3),run(63,64,1,8)],
        vec![run(-64,53,1,4),run(53,63,4,15)],
        vec![run(-64,63,1,11)],
        vec![run(-64,53,1,9),run(54,63,2,15)],
        vec![run(-64,53,1,9),run(53,54,0,15),run(54,63,2,15)],
    ] {
        let original=runs.clone();
        shade_open_water(&mut runs,&blocks());
        assert_eq!(runs,original);
    }
}

#[test]
fn open_water_below_zero_uses_depth_not_absolute_height() {
    let mut runs=vec![run(-64,-30,1,15),run(-30,-26,2,15)];
    shade_open_water(&mut runs,&blocks());
    assert_eq!(runs[0].sky,11);
    assert_eq!(runs[1].sky,15);
}

#[test]
fn retaining_aquatic_states_keeps_one_continuous_water_volume() {
    let blocks=blocks();
    let settings=RenderSettings{cave_culling:false,cave_culling_height:60,min_y:-64};
    let mesh=|plant:bool,hidden:bool| {
        let mut source=FullDataSource::empty(pos::encode(6,0,0));
        for x in 0..3 {for z in 0..3 {
            let runs=vec![run(-64,53,1,5),run(53,57,if plant&&x==1&&z==1 {3}else{2},9),run(57,63,2,15)];
            let values=column_from_runs(&mut source.mapping,&runs,-64,320);
            source.set_column(x,z,values,gen_step::FEATURES);
        }}
        let render=RenderSource::from_full(&source,&blocks,&settings);
        build_quads(&render,[None;4],&settings,|x,z|hidden&&x==1&&z==1)
    };
    for hidden in [false,true] {
        let plain=mesh(false,hidden);let planted=mesh(true,hidden);
        let geometry=|quads:Vec<distant_horizons_for_horizon_client::Quad>|{
            let mut q:Vec<_>=quads.into_iter().map(|q|(q.dir,q.plane,q.u0,q.v0,q.u1,q.v1,q.transparent,q.lower_top)).collect();
            q.sort_unstable();q
        };
        assert_eq!(geometry(plain),geometry(planted),"plants add neither inner water walls nor holes; hidden={hidden}");
    }
}
