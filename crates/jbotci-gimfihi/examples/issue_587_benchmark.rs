use std::hint::black_box;
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use bityzba::ensures;
use bityzba::requires;
use jbotci_gimfihi::{
    CollisionScope, GimfihiOutput, GimfihiPreset, GimfihiRequest, GimfihiScorer, compose_gismu,
    default_shapes, parse_source_spec,
};
use jbotci_phonetic::AlineParameters;

#[requires(true)]
#[ensures(true)]
fn main() {
    let iterations = std::env::args()
        .nth(1)
        .map(|value| {
            value
                .parse::<usize>()
                .expect("iteration count must be an integer")
        })
        .unwrap_or(10);
    assert!(iterations > 0, "iteration count must be positive");
    let dictionary = jbotci_dictionary_data::english();
    let request = issue_587_request();

    let warm_up = black_box(compose_gismu(dictionary, &request).expect("warm-up request"));
    let expected_signature = output_signature(&warm_up);
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let output = black_box(compose_gismu(dictionary, &request).expect("benchmark request"));
        samples.push(start.elapsed());
        assert_eq!(
            output_signature(&output),
            expected_signature,
            "benchmark output changed between iterations"
        );
    }
    samples.sort_unstable();
    let middle = samples.len() / 2;
    let median_ms = if samples.len() % 2 == 0 {
        (duration_ms(samples[middle - 1]) + duration_ms(samples[middle])) / 2.0
    } else {
        duration_ms(samples[middle])
    };
    let mean_ms = samples.iter().copied().map(duration_ms).sum::<f64>() / iterations as f64;
    let top = warm_up
        .candidates
        .iter()
        .take(3)
        .map(|candidate| (candidate.word.as_str(), candidate.score.to_bits()))
        .collect::<Vec<_>>();
    println!(
        "output_signature={expected_signature:016x} candidate_count={} filtered_count={} winner={:?} top={top:?}",
        warm_up.candidate_count, warm_up.filtered_count, warm_up.winner
    );
    println!("iterations={iterations} median_ms={median_ms:.3} mean_ms={mean_ms:.3}");
}

#[requires(true)]
#[ensures(true)]
fn output_signature(output: &GimfihiOutput) -> u64 {
    // Hash the complete Debug projection rather than only the displayed top K
    // so the before/after benchmark mechanically detects output-shape or
    // detail changes. FNV-1a is fixed here to keep the signature reproducible.
    format!("{output:?}")
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

#[requires(true)]
#[ensures(ret.scorer == GimfihiScorer::Phonetic)]
fn issue_587_request() -> GimfihiRequest {
    let source_specs = [
        "cmn::[fat͡ɕjɑʊ]",
        "eng::[fɚmɛnt]",
        "spa::[feɾment]",
        "hin::[kɪɳʋan]",
        "ara::[taxamːur]",
        "ben::[ɡãd͡ʒɔn]",
        "rus::[fʲermʲent]",
        "por::[feʁmẽt]",
        "msa::[pɘnapaian]",
        "jpn::[hakːoː]",
        "deu::[ɡɛːʁ]",
        "fra::[fɛʁmɑ̃t]",
    ];
    GimfihiRequest {
        scorer: GimfihiScorer::Phonetic,
        phonetic_parameters: AlineParameters::default(),
        preset: Some(GimfihiPreset::Ilmen12),
        sources: source_specs
            .into_iter()
            .map(|spec| parse_source_spec(spec).expect("valid source"))
            .collect(),
        shapes: default_shapes(),
        all_letters: false,
        check_collisions: CollisionScope::All,
        show_collisions: false,
        require_free_short_rafsi: false,
        count: 160,
        highlight: None,
    }
}

#[requires(true)]
#[ensures(ret.is_finite() && ret >= 0.0)]
fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
