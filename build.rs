use pipl::*;

fn main() {
    // After Effects / Premiere が読む PiPL リソースを生成する
    pipl::plugin_build(vec![
        Property::Kind(PIPLType::AEEffect),
        Property::Name("SVG Stroke"),
        Property::Category("Stylize"),
        #[cfg(target_os = "windows")]
        Property::CodeWin64X86("EffectMain"),
        #[cfg(target_os = "macos")]
        Property::CodeMacIntel64("EffectMain"),
        #[cfg(target_os = "macos")]
        Property::CodeMacARM64("EffectMain"),
        Property::AE_PiPL_Version { major: 2, minor: 0 },
        Property::AE_Effect_Spec_Version { major: 13, minor: 28 },
        Property::AE_Effect_Version { version: 1, subversion: 0, bugversion: 0, stage: Stage::Develop, build: 1 },
        Property::AE_Effect_Info_Flags(0),
        // PF_OutFlag_PIX_INDEPENDENT | PF_OutFlag_USE_OUTPUT_EXTENT | PF_OutFlag_I_EXPAND_BUFFER
        Property::AE_Effect_Global_OutFlags(OutFlags::PixIndependent | OutFlags::UseOutputExtent | OutFlags::IExpandBuffer),
        Property::AE_Effect_Global_OutFlags_2(OutFlags2::FloatColorAware | OutFlags2::SupportsThreadedRendering),
        Property::AE_Effect_Match_Name("JP SVG_Stroke"),
    ]);
}
