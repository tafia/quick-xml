use phf::phf_map;

#[derive(Debug)]
pub struct QuickXmlMeta {
    pub namespace_declarations: &'static [(&'static str, &'static str)],
    pub identifier_prefix_map: phf::Map<&'static str, &'static str>
}
pub trait CurrentItemVisitorQXmlMeta {
    fn visit_current_item_as_self<T>(
        &self,
        contained_visitor: &mut T,
        ident_in_parent: Option<&'static str>,
        parent_meta: Option<&'static QuickXmlMeta>,
    ) -> &'static QuickXmlMeta
    where
        T: ContainedItemVisitorQXmlMeta;
}

impl CurrentItemVisitorQXmlMeta for String {
    fn visit_current_item_as_self<T>(
        &self,
        contained_visitor: &mut T,
        ident_in_parent: Option<&'static str>,
        parent_meta: Option<&'static QuickXmlMeta>,
    ) -> &'static QuickXmlMeta
    where
        T: ContainedItemVisitorQXmlMeta
    {
        &QuickXmlMeta {
            namespace_declarations: &[],
            identifier_prefix_map: phf_map! {},
        }
    }
}

impl CurrentItemVisitorQXmlMeta for u32 {
    fn visit_current_item_as_self<T>(
        &self,
        contained_visitor: &mut T,
        ident_in_parent: Option<&'static str>,
        parent_meta: Option<&'static QuickXmlMeta>,
    ) -> &'static QuickXmlMeta
    where
        T: ContainedItemVisitorQXmlMeta
    {
        &QuickXmlMeta {
            namespace_declarations: &[],
            identifier_prefix_map: phf_map! {},
        }
    }
}

impl<X: CurrentItemVisitorQXmlMeta> CurrentItemVisitorQXmlMeta for Vec<X> {
    fn visit_current_item_as_self<T>(
        &self,
        contained_visitor: &mut T,
        ident_in_parent: Option<&'static str>,
        parent_meta: Option<&'static QuickXmlMeta>,
    ) -> &'static QuickXmlMeta
    where
        T: ContainedItemVisitorQXmlMeta
    {
        let self_meta = &QuickXmlMeta {
            namespace_declarations: &[],
            identifier_prefix_map: phf_map! {},
        };
        self.iter().for_each(|item| T::visit_contained_item(contained_visitor, item, ident_in_parent, parent_meta, true));
        self_meta
    }
}

pub trait ContainedItemVisitorQXmlMeta {
    fn visit_contained_item<T: CurrentItemVisitorQXmlMeta>(
        &mut self,
        obj_to_ser: &T,
        ident_in_parent: Option<&'static str>,
        parent_meta: Option<&'static QuickXmlMeta>,
        is_pseudoobject: bool,
    );
} 