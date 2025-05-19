use crate::scene::scene::SceneBufferData;
use std::iter::Peekable;
use std::ops::Range;
use std::vec::IntoIter;
#[derive(PartialEq, PartialOrd)]
enum IndexResult {
    StrictlyGreater,
    StrictlyLess,
    FullyContained,
    FullyCover,
    LeftOverlap,
    RightOverlap,
}
fn get_index_result(
    current_range: &Range<usize>,
    primitive_indices_range: &Range<usize>,
) -> IndexResult {
    if primitive_indices_range.start > current_range.end + 1 {
        IndexResult::StrictlyGreater
    } else if primitive_indices_range.end + 1 < current_range.start {
        IndexResult::StrictlyLess
    } else if primitive_indices_range.start == current_range.end + 1 {
        // we consider this case a right overlap, because the result is the same
        IndexResult::RightOverlap
    } else if primitive_indices_range.end + 1 == current_range.start {
        // consider this a left overlap
        IndexResult::LeftOverlap
    } else if primitive_indices_range.start > current_range.start
        && primitive_indices_range.end > current_range.end
    {
        IndexResult::RightOverlap
    } else if primitive_indices_range.start < current_range.start
        && primitive_indices_range.end < current_range.end
    {
        IndexResult::LeftOverlap
    } else if primitive_indices_range.start > current_range.start
        && primitive_indices_range.end < current_range.end
    {
        IndexResult::FullyContained
    } else if primitive_indices_range.start < current_range.start
        && primitive_indices_range.end > current_range.end
    {
        IndexResult::FullyContained
    } else {
        panic!()
    }
}

fn define_index_ranges(
    range_vec: &mut Vec<Range<usize>>,
    range_iter: &mut Peekable<IntoIter<Range<usize>>>,
    primitive_indices_range: std::ops::Range<usize>,
) {
    let mut current_idx: usize = 0;
    'outer: loop {
        if let Some(current_range) = range_iter.next() {
            let index_result = get_index_result(&current_range, &primitive_indices_range);
            use IndexResult::*;
            match index_result {
                // if this range is strictly greater than the current range, and this is the last
                // range, add a new range to the end, unless we can combine it into a continuous
                // new range
                StrictlyGreater => {
                    if range_iter.peek().is_none() {
                        range_vec.push(std::ops::Range {
                            start: primitive_indices_range.start,
                            end: primitive_indices_range.end,
                        });
                        break 'outer;
                    }
                    current_idx += 1;
                    continue 'outer;
                }
                // if this range is strictly less than the curent range it needs to be placed
                // before the current range
                StrictlyLess => {
                    range_vec.insert(current_idx, primitive_indices_range.clone());
                }

                // if the primitive range overlaps the left side of this current range, replace the
                // current range with a new one that spans backwards to include the start of this
                // primitives range
                LeftOverlap => {
                    range_vec.splice(
                        current_idx..current_idx,
                        vec![primitive_indices_range.start..current_range.end],
                    );
                }
                // do nothing, we are already covered
                FullyContained => {}
                // if this primirives range fully encapsulates the current range, or it overlaps
                // it on the right side, the end index may still overlap with some of the other
                // ranges. loop through until we either
                // 1. hit the end of the range vec -> replace all subsequent ranges with new
                // 2. find a left overlap, splice in a vec which spans from the start of the vec in
                //    which we first found to the end of the new one
                FullyCover | RightOverlap => {
                    if range_iter.peek().is_none() {
                        break 'outer;
                    }

                    let mut splice_offset = 0;
                    'inner: loop {
                        if range_iter.peek().is_none() {
                            range_vec.splice(
                                current_idx..current_idx + splice_offset,
                                vec![current_range.start..primitive_indices_range.end],
                            );
                        }
                        let next_range = range_iter.next().unwrap();
                        splice_offset += 1;
                        let next_index_result =
                            get_index_result(&next_range, &primitive_indices_range);
                        if next_index_result == FullyCover {
                            continue 'inner;
                        }
                        if next_index_result == LeftOverlap {
                            if index_result == FullyCover {
                                range_vec.splice(
                                    current_idx..current_idx + splice_offset,
                                    vec![primitive_indices_range.start..next_range.end],
                                );
                            } else {
                                range_vec.splice(
                                    current_idx..current_idx + splice_offset,
                                    vec![current_range.start..next_range.end],
                                );
                            }
                            break 'outer;
                        } else if next_index_result == StrictlyLess {
                            if index_result == FullyCover {
                                range_vec.splice(
                                    current_idx..current_idx + splice_offset - 1,
                                    vec![
                                        primitive_indices_range.start..primitive_indices_range.end,
                                    ],
                                );
                            } else {
                                range_vec.splice(
                                    current_idx..current_idx + splice_offset - 1,
                                    vec![current_range.start..primitive_indices_range.end],
                                );
                            }
                            break 'outer;
                        } else {
                            panic!();
                        }
                    }
                }
            }
        }
        current_idx += 1;
    }
}
#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_new_strictly_greater() {
        let mut indices_ranges = Vec::<Range<usize>>::new();
        indices_ranges.push(0..4);
        indices_ranges.push(7..10);
        let mut range_iter = indices_ranges.clone().into_iter().peekable();
        let primitive_indices_range = Range { start: 12, end: 15 };
        define_index_ranges(
            &mut indices_ranges,
            &mut range_iter,
            primitive_indices_range,
        );

        assert_eq!(indices_ranges.len(), 3);
        assert_eq!(
            *indices_ranges.last().unwrap(),
            Range { start: 12, end: 15 }
        );
    }

    #[test]
    fn test_new_right_overlap_1() {
        let mut indices_ranges = Vec::<Range<usize>>::new();
        indices_ranges.push(0..4);
        indices_ranges.push(7..10);
        let mut range_iter = indices_ranges.clone().into_iter().peekable();
        let primitive_indices_range = Range { start: 12, end: 15 };
    }
}
