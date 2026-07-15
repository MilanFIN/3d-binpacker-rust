use crate::common::box_spec::BinBox;

pub fn export_csv(bins: &[Vec<BinBox>]) -> String {
    let mut csv = String::from("Bin,Box,x, y, z, w ,h ,d \n");
    for (i, bin) in bins.iter().enumerate() {
        for box_item in bin.iter() {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                i,
                box_item.id,
                box_item.position.x,
                box_item.position.y,
                box_item.position.z,
                box_item.size.x,
                box_item.size.y,
                box_item.size.z
            ));
        }
    }
    csv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::point3f::Point3f;

    #[test]
    fn test_export_csv() {
        let b1 = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(2.0, 3.0, 4.0));
        let b2 = BinBox::new_without_weight(2, Point3f::new(1.0, 1.0, 1.0), Point3f::new(2.0, 3.0, 4.0));
        let bins = vec![vec![b1, b2]];
        let csv = export_csv(&bins);
        let expected = "Bin,Box,x, y, z, w ,h ,d \n0,1,0,0,0,2,3,4\n0,2,1,1,1,2,3,4\n";
        assert_eq!(csv, expected);
    }
}
