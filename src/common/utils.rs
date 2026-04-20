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
