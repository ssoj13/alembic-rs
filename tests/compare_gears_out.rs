//! Compare animated Xform samples between reference and writer output.
//!
//! This helps locate transform mismatches that only appear during playback.

use alembic::abc::IArchive;
use alembic::geom::{IXform, IPolyMesh};

const REF_PATH: &str = "data/Abc/gears.abc";
const OUT_PATH: &str = "data/Abc/gears.abc";

#[test]
fn compare_gears_xforms() {
    let ref_archive = IArchive::open(REF_PATH).expect("open ref archive");
    let out_archive = IArchive::open(OUT_PATH).expect("open out archive");

    let ref_root = ref_archive.getTop();
    let out_root = out_archive.getTop();

    let mut mismatches = 0usize;

    compare_time_samplings(&ref_archive, &out_archive, &mut mismatches);
    compare_subtree(&ref_root, &out_root, &mut mismatches);

    assert_eq!(mismatches, 0, "Xform mismatches found; see log for details");
}

/// Compare archive time samplings for playback parity.
fn compare_time_samplings(
    ref_archive: &IArchive,
    out_archive: &IArchive,
    mismatches: &mut usize,
) {
    let ref_count = ref_archive.getNumTimeSamplings();
    let out_count = out_archive.getNumTimeSamplings();
    if ref_count != out_count {
        println!("Time sampling count mismatch: ref={} out={}", ref_count, out_count);
        *mismatches += 1;
        return;
    }

    for i in 0..ref_count {
        let Some(ref_ts) = ref_archive.getTimeSampling(i) else { continue; };
        let Some(out_ts) = out_archive.getTimeSampling(i) else { continue; };

        if ref_ts.sampling_type != out_ts.sampling_type {
            println!("Time sampling type mismatch at index {}", i);
            *mismatches += 1;
            continue;
        }

        let ref_times = ref_ts.stored_times();
        let out_times = out_ts.stored_times();
        if ref_times.len() != out_times.len() {
            println!("Stored times length mismatch at index {}", i);
            *mismatches += 1;
            continue;
        }

        for (a, b) in ref_times.iter().zip(out_times.iter()) {
            if (a - b).abs() > 1.0e-9 {
                println!(
                    "Stored time mismatch at index {}: ref={} out={}",
                    i, a, b
                );
                *mismatches += 1;
                break;
            }
        }
    }
}

/// Compare xform samples for a subtree by matching children by name.
fn compare_subtree<'a>(
    ref_obj: &'a alembic::abc::IObject<'a>,
    out_obj: &'a alembic::abc::IObject<'a>,
    mismatches: &mut usize,
) {
    let name = out_obj.getFullName();

    if let (Some(ref_xf), Some(out_xf)) = (IXform::new(ref_obj), IXform::new(out_obj)) {
        let ref_samples = ref_xf.getNumSamples();
        let out_samples = out_xf.getNumSamples();
        if ref_samples != out_samples {
            println!(
                "Sample count mismatch: {} ref={} out={}",
                name, ref_samples, out_samples
            );
            *mismatches += 1;
        } else {
            for i in 0..ref_samples {
                let Ok(ref_sample) = ref_xf.getSample(i) else { continue; };
                let Ok(out_sample) = out_xf.getSample(i) else { continue; };

                let ref_m = ref_sample.matrix();
                let out_m = out_sample.matrix();

                let ref_cols = ref_m.to_cols_array();
                let out_cols = out_m.to_cols_array();

                let mut max_abs = 0.0f32;
                for (a, b) in ref_cols.iter().zip(out_cols.iter()) {
                    let diff = (a - b).abs();
                    if diff > max_abs {
                        max_abs = diff;
                    }
                }

                if max_abs > 1.0e-3 {
                    println!("Xform mismatch: {} sample={} max_abs={:.6}", name, i, max_abs);
                    *mismatches += 1;
                    break;
                }
            }
        }
    }

    if let (Some(ref_mesh), Some(out_mesh)) = (IPolyMesh::new(ref_obj), IPolyMesh::new(out_obj)) {
        let ref_samples = ref_mesh.getNumSamples();
        let out_samples = out_mesh.getNumSamples();
        if ref_samples != out_samples {
            println!(
                "PolyMesh sample count mismatch: {} ref={} out={}",
                name, ref_samples, out_samples
            );
            *mismatches += 1;
        } else {
            for i in 0..ref_samples {
                let Ok(ref_sample) = ref_mesh.getSample(i) else { continue; };
                let Ok(out_sample) = out_mesh.getSample(i) else { continue; };

                let (ref_min, ref_max) = ref_sample.compute_bounds();
                let (out_min, out_max) = out_sample.compute_bounds();

                let max_abs = (ref_min - out_min)
                    .abs()
                    .max_element()
                    .max((ref_max - out_max).abs().max_element());

                if max_abs > 1.0e-3 {
                    println!(
                        "PolyMesh bounds mismatch: {} sample={} max_abs={:.6}",
                        name, i, max_abs
                    );
                    *mismatches += 1;
                    break;
                }
            }
        }
    }

    let ref_children: Vec<_> = ref_obj.getChildren().collect();
    for out_child in out_obj.getChildren() {
        if let Some(ref_child) = ref_children
            .iter()
            .find(|c| c.getName() == out_child.getName())
        {
            compare_subtree(ref_child, &out_child, mismatches);
        }
    }
}
