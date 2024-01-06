pub trait Buffer {
    fn as_slice(&self) -> &[u8];

    fn as_slice_mut(&mut self) -> &mut [u8];

    #[allow(unused_variables)]
    fn grow(&mut self, new_size: usize) {
        // noop, can't grow
    }

    fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    fn len(&self) -> usize {
        self.as_slice().len()
    }
}

impl<const SIZE: usize> Buffer for [u8; SIZE] {
    fn as_slice(&self) -> &[u8] {
        self
    }

    fn as_slice_mut(&mut self) -> &mut [u8] {
        self
    }
}

impl<'a> Buffer for &'a mut [u8] {
    fn as_slice(&self) -> &[u8] {
        self
    }

    fn as_slice_mut(&mut self) -> &mut [u8] {
        self
    }
}

impl Buffer for () {
    fn as_slice(&self) -> &[u8] {
        &[]
    }

    fn as_slice_mut(&mut self) -> &mut [u8] {
        &mut []
    }
}
