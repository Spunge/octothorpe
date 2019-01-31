

struct Introduction {
    time: u32,
    bytes: [u8; 6],
}

struct Inquiry {
    time: u32,
    bytes: [u8; 12],
}

enum Message {
    Introduction(Introduction),
    Inquiry(Inquiry),
}
