
#[test]
fn test_open() {
    let r = XmlReader::from_file("/home/jtuffe/download/xmls/\
        EFVPnL_G_EXO_HYB_20160113_20160114.xml").unwrap();
//         EFVPnL_G_REL_TRD_OTH_Explained_20160111_20160112.xml").unwrap();
//         EFVPnL_D_GY-ESO-PIVOT_Explained_20160113_20160114.xml").unwrap();
    for e in r {
        assert!(e.is_ok());
        println!("{:?}", e);
    }
}
   
