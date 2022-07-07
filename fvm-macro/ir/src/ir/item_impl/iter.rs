pub struct IterConstructors<'a> {
    item_impl: &'a item_impl::ItemImpl,
    impl_items: core::slice::Iter<'a, ImplItem>,
}

impl<'a> IterConstructors<'a> {
    pub(super) fn new(item_impl: &'a ItemImpl) -> Self {
        Self {
            item_impl,
            impl_items: item_impl.items.iter(),
        }
    }
}

impl<'a> Iterator for IterConstructors<'a> {
    type Item = CallableWithSelector<'a, constructor::Constructor>;

    fn next(&mut self) -> Option<Self::Item> {
        'repeat: loop {
            match self.impl_items.next() {
                None => return None,
                Some(impl_item) => {
                    if let Some(constructor) = impl_item.filter_map_constructor() {
                        return Some(CallableWithSelector::new(
                            self.item_impl,
                            constructor,
                        ));
                    }
                    continue 'repeat;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct IterMessages<'a> {
    item_impl: &'a item_impl::ItemImpl,
    impl_items: core::slice::Iter<'a, ImplItem>,
}

impl<'a> IterMessages<'a> {
    pub(super) fn new(item_impl: &'a ItemImpl) -> Self {
        Self {
            item_impl,
            impl_items: item_impl.items.iter(),
        }
    }
}

impl<'a> Iterator for IterMessages<'a> {
    type Item = CallableWithSelector<'a, message::Message>;

    fn next(&mut self) -> Option<Self::Item> {
        'repeat: loop {
            match self.impl_items.next() {
                None => return None,
                Some(impl_item) => {
                    if let Some(message) = impl_item.filter_map_message() {
                        return Some(CallableWithSelector::new(self.item_impl, message));
                    }
                    continue 'repeat;
                }
            }
        }
    }
}
