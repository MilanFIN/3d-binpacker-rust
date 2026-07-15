use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::common::point3f::Point3f;
use crate::common::space::Space;

pub struct PlacementUtils;

impl PlacementUtils {
    pub fn unordered_remove_space(bin: &mut Bin, space_index: usize) {
        bin.free_spaces.swap_remove(space_index);
    }

    pub fn find_fit(box_item: &BinBox, space: &Space, rotations: Option<&[i32]>) -> Option<BinBox> {
        // Always check default orientation (x, y, z)
        if box_item.size.x <= space.w && box_item.size.y <= space.h && box_item.size.z <= space.d {
            return Some(box_item.clone());
        }

        if let Some(rots) = rotations {
            if rots.is_empty() {
                return None;
            }

            let check_x = rots.contains(&0);
            let check_y = rots.contains(&1);
            let check_z = rots.contains(&2);

            if check_x {
                if box_item.size.x <= space.w
                    && box_item.size.z <= space.h
                    && box_item.size.y <= space.d
                {
                    return Some(BinBox::new(
                        box_item.id,
                        box_item.position,
                        Point3f::new(box_item.size.x, box_item.size.z, box_item.size.y),
                        box_item.weight,
                    ));
                }
            }
            if check_y {
                if box_item.size.y <= space.w
                    && box_item.size.x <= space.h
                    && box_item.size.z <= space.d
                {
                    return Some(BinBox::new(
                        box_item.id,
                        box_item.position,
                        Point3f::new(box_item.size.y, box_item.size.x, box_item.size.z),
                        box_item.weight,
                    ));
                }
            }
            if check_z {
                if box_item.size.z <= space.w
                    && box_item.size.y <= space.h
                    && box_item.size.x <= space.d
                {
                    return Some(BinBox::new(
                        box_item.id,
                        box_item.position,
                        Point3f::new(box_item.size.z, box_item.size.y, box_item.size.x),
                        box_item.weight,
                    ));
                }
            }
        }

        None
    }

    pub fn place_box_bsp(box_item: &BinBox, bin: &mut Bin, space_index: usize) {
        let space = bin.free_spaces[space_index].clone();

        let mut placed_box = BinBox::new(
            box_item.id,
            Point3f::new(space.x, space.y, space.z),
            Point3f::new(box_item.size.x, box_item.size.y, box_item.size.z),
            box_item.weight,
        );
        bin.boxes.push(placed_box.clone());
        bin.weight += placed_box.weight;

        bin.free_spaces.remove(space_index);

        let right = Space::new(
            space.x + box_item.size.x,
            space.y,
            space.z,
            space.w - box_item.size.x,
            space.h,
            space.d,
        );
        let top = Space::new(
            space.x,
            space.y + box_item.size.y,
            space.z,
            box_item.size.x,
            space.h - box_item.size.y,
            space.d,
        );
        let front = Space::new(
            space.x,
            space.y,
            space.z + box_item.size.z,
            box_item.size.x,
            box_item.size.y,
            space.d - box_item.size.z,
        );

        if right.w > 0.0 && right.h > 0.0 && right.d > 0.0 {
            bin.free_spaces.push(right);
        }
        if top.w > 0.0 && top.h > 0.0 && top.d > 0.0 {
            bin.free_spaces.push(top);
        }
        if front.w > 0.0 && front.h > 0.0 && front.d > 0.0 {
            bin.free_spaces.push(front);
        }
    }

    pub fn place_box_bsp_2d(box_item: &BinBox, bin: &mut Bin, space_index: usize) {
        let space = bin.free_spaces[space_index].clone();

        let mut placed_box = BinBox::new(
            box_item.id,
            Point3f::new(space.x, space.y, space.z),
            Point3f::new(box_item.size.x, box_item.size.y, box_item.size.z),
            box_item.weight,
        );
        bin.boxes.push(placed_box.clone());
        bin.weight += placed_box.weight;

        bin.free_spaces.remove(space_index);

        let right = Space::new(
            space.x + box_item.size.x,
            space.y,
            space.z,
            space.w - box_item.size.x,
            space.h,
            space.d,
        );
        let top = Space::new(
            space.x,
            space.y + box_item.size.y,
            space.z,
            box_item.size.x,
            space.h - box_item.size.y,
            space.d,
        );

        if right.w > 0.0 && right.h > 0.0 && right.d > 0.0 {
            bin.free_spaces.push(right);
        }
        if top.w > 0.0 && top.h > 0.0 && top.d > 0.0 {
            bin.free_spaces.push(top);
        }
    }

    pub fn place_box_ems(box_item: &BinBox, bin: &mut Bin, space_index: usize) -> BinBox {
        let space = bin.free_spaces[space_index].clone();

        let mut placed_box = BinBox::new(
            box_item.id,
            Point3f::new(space.x, space.y, space.z),
            Point3f::new(box_item.size.x, box_item.size.y, box_item.size.z),
            box_item.weight,
        );
        bin.boxes.push(placed_box.clone());
        bin.weight += placed_box.weight;

        Self::unordered_remove_space(bin, space_index);

        let right = Space::new(
            space.x + box_item.size.x,
            space.y,
            space.z,
            space.w - box_item.size.x,
            space.h,
            space.d,
        );
        let top = Space::new(
            space.x,
            space.y + box_item.size.y,
            space.z,
            space.w,
            space.h - box_item.size.y,
            space.d,
        );
        let front = Space::new(
            space.x,
            space.y,
            space.z + box_item.size.z,
            space.w,
            space.h,
            space.d - box_item.size.z,
        );

        if right.w > 0.0 && right.h > 0.0 && right.d > 0.0 {
            bin.free_spaces.push(right);
        }
        if top.w > 0.0 && top.h > 0.0 && top.d > 0.0 {
            bin.free_spaces.push(top);
        }
        if front.w > 0.0 && front.h > 0.0 && front.d > 0.0 {
            bin.free_spaces.push(front);
        }

        placed_box
    }

    pub fn place_box_ems_and_return_new_spaces(
        box_item: &BinBox,
        bin: &mut Bin,
        space_index: usize,
    ) -> Vec<Space> {
        let space = bin.free_spaces[space_index].clone();

        let mut placed_box = BinBox::new(
            box_item.id,
            Point3f::new(space.x, space.y, space.z),
            Point3f::new(box_item.size.x, box_item.size.y, box_item.size.z),
            box_item.weight,
        );
        bin.boxes.push(placed_box.clone());

        Self::unordered_remove_space(bin, space_index);

        let mut new_free_spaces = Vec::new();

        let right = Space::new(
            space.x + box_item.size.x,
            space.y,
            space.z,
            space.w - box_item.size.x,
            space.h,
            space.d,
        );
        let top = Space::new(
            space.x,
            space.y + box_item.size.y,
            space.z,
            space.w,
            space.h - box_item.size.y,
            space.d,
        );
        let front = Space::new(
            space.x,
            space.y,
            space.z + box_item.size.z,
            space.w,
            space.h,
            space.d - box_item.size.z,
        );

        if right.w > 0.0 && right.h > 0.0 && right.d > 0.0 {
            new_free_spaces.push(right);
        }
        if top.w > 0.0 && top.h > 0.0 && top.d > 0.0 {
            new_free_spaces.push(top);
        }
        if front.w > 0.0 && front.h > 0.0 && front.d > 0.0 {
            new_free_spaces.push(front);
        }

        new_free_spaces
    }

    pub fn prune_colliding_spaces_ems(box_item: &BinBox, bin: &mut Bin) {
        let mut i = bin.free_spaces.len() as isize - 1;
        while i >= 0 {
            let idx = i as usize;
            let space = bin.free_spaces[idx].clone();
            if box_item.collides_with_space(&space) {
                Self::unordered_remove_space(bin, idx);
                Self::split_colliding_free_space_ems(box_item, &space, bin);
            }
            i -= 1;
        }
    }

    pub fn split_colliding_free_space_ems(box_item: &BinBox, space: &Space, bin: &mut Bin) {
        // 1. Right
        if box_item.position.x + box_item.size.x < space.x + space.w {
            bin.free_spaces.push(Space::new(
                box_item.position.x + box_item.size.x,
                space.y,
                space.z,
                (space.x + space.w) - (box_item.position.x + box_item.size.x),
                space.h,
                space.d,
            ));
        }
        // 2. Left
        if box_item.position.x > space.x {
            bin.free_spaces.push(Space::new(
                space.x,
                space.y,
                space.z,
                box_item.position.x - space.x,
                space.h,
                space.d,
            ));
        }
        // 3. Top
        if box_item.position.y + box_item.size.y < space.y + space.h {
            bin.free_spaces.push(Space::new(
                space.x,
                box_item.position.y + box_item.size.y,
                space.z,
                space.w,
                (space.y + space.h) - (box_item.position.y + box_item.size.y),
                space.d,
            ));
        }
        // 4. Bottom
        if box_item.position.y > space.y {
            bin.free_spaces.push(Space::new(
                space.x,
                space.y,
                space.z,
                space.w,
                box_item.position.y - space.y,
                space.d,
            ));
        }
        // 5. Front
        if box_item.position.z + box_item.size.z < space.z + space.d {
            bin.free_spaces.push(Space::new(
                space.x,
                space.y,
                box_item.position.z + box_item.size.z,
                space.w,
                space.h,
                (space.z + space.d) - (box_item.position.z + box_item.size.z),
            ));
        }
        // 6. Back
        if box_item.position.z > space.z {
            bin.free_spaces.push(Space::new(
                space.x,
                space.y,
                space.z,
                space.w,
                space.h,
                box_item.position.z - space.z,
            ));
        }
    }

    pub fn prune_wrapped_spaces_bin_ems(bin: &mut Bin) {
        let mut i = bin.free_spaces.len() as isize - 1;
        while i >= 0 {
            let idx = i as usize;
            let space1 = bin.free_spaces[idx].clone();

            if space1.w <= 0.0 || space1.h <= 0.0 || space1.d <= 0.0 {
                Self::unordered_remove_space(bin, idx);
                i -= 1;
                continue;
            }

            let mut is_wrapped = false;
            let mut j = bin.free_spaces.len() as isize - 1;
            while j >= 0 {
                if i == j {
                    j -= 1;
                    continue;
                }
                let space2 = &bin.free_spaces[j as usize];

                if space1.x >= space2.x
                    && space1.y >= space2.y
                    && space1.z >= space2.z
                    && (space1.x + space1.w) <= (space2.x + space2.w)
                    && (space1.y + space1.h) <= (space2.y + space2.h)
                    && (space1.z + space1.d) <= (space2.z + space2.d)
                {
                    is_wrapped = true;
                    break;
                }
                j -= 1;
            }

            if is_wrapped {
                Self::unordered_remove_space(bin, idx);
            }
            i -= 1;
        }
    }

    pub fn calculate_score_ems(_box_item: &BinBox, space: &Space) -> f32 {
        space.x + space.y + space.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_fit() {
        let box_item = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(2.0, 3.0, 4.0));
        let space = Space::new(0.0, 0.0, 0.0, 3.0, 4.0, 5.0);
        
        // Exact fit without rotation
        let fitted = PlacementUtils::find_fit(&box_item, &space, None);
        assert!(fitted.is_some());
        
        let space2 = Space::new(0.0, 0.0, 0.0, 3.0, 2.0, 5.0);
        let fitted2 = PlacementUtils::find_fit(&box_item, &space2, None);
        assert!(fitted2.is_none());

        // Fit with rotation
        let fitted3 = PlacementUtils::find_fit(&box_item, &space2, Some(&vec![0, 1, 2]));
        assert!(fitted3.is_some()); // y rotation: 3.0 x 2.0 x 4.0 (will fit in 3x2x5)
    }

    #[test]
    fn test_place_box_bsp() {
        let mut bin = Bin::new(0, 10.0, 10.0, 10.0);
        let box_item = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0));
        
        PlacementUtils::place_box_bsp(&box_item, &mut bin, 0);
        
        assert_eq!(bin.boxes.len(), 1);
        assert_eq!(bin.free_spaces.len(), 3); // right, top, front
    }
}
