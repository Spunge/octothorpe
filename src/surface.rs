
pub struct Surface {
    // TODO . what to update when?
    // TODO - what are we viewing?
    is_viewing: bool,
}

impl Surface {
    pub fn new() -> Self {
        Surface { is_viewing: true }
    }
}
