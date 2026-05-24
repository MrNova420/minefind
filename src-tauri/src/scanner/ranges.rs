use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub struct CidrRange {
    pub name: String,
    pub start: Ipv4Addr,
    pub end: Ipv4Addr,
    pub mask: u8,
}

pub fn get_full_ipv4_ranges() -> Vec<CidrRange> {
    let reserved: &[u8] = &[0, 10, 127, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255];
    let mut ranges = Vec::new();
    for first_octet in 0u8..=255u8 {
        if reserved.contains(&first_octet) { continue; }
        // 100.64.0.0/10 (CGNAT) - skip full /8
        if first_octet == 100 { continue; }
        ranges.push(CidrRange {
            name: format!("{}.0.0.0/8", first_octet),
            start: ip(first_octet, 0, 0, 0),
            end: ip(first_octet, 255, 255, 255),
            mask: 8,
        });
    }
    ranges
}

pub fn get_priority_ranges() -> Vec<CidrRange> {
    get_hosting_ranges()
}

pub fn get_ipv6_ranges() -> Vec<String> {
    vec![
        "2a01:4f8::/32".into(),
        "2a01:4f9::/32".into(),
        "2a03:4000::/29".into(),
        "2001:41d0::/32".into(),
        "2001:41d1::/32".into(),
        "2604:a880::/32".into(),
        "2400:6180::/32".into(),
        "2a04:3540::/32".into(),
        "2001:19f0::/32".into(),
        "2600:3c00::/32".into(),
        "2a01:7e00::/32".into(),
        "2400:cb00::/32".into(),
        "2a05:d014::/32".into(),
        "2600:1f18::/32".into(),
        "2a02:7aa0::/32".into(),
        "2a02:c206::/32".into(),
    ]
}

pub fn is_reserved(ip_u32: u32) -> bool {
    if ip_u32 < 0x0100_0000 { return true; }
    if (ip_u32 >> 24) == 10 { return true; }
    if (ip_u32 & 0xFFC0_0000) == 0x6440_0000 { return true; }
    if (ip_u32 >> 24) == 127 { return true; }
    if (ip_u32 >> 16) == 0xA9FE { return true; }
    if (ip_u32 >> 20) == 0xAC1 { return true; }
    if (ip_u32 >> 16) == 0xC0A8 { return true; }
    if (ip_u32 >> 16) == 0xC612 || (ip_u32 >> 16) == 0xC613 { return true; }
    if (ip_u32 & 0xF000_0000) == 0xE000_0000 { return true; }
    false
}

pub fn prefix_for_ip(ip_u32: u32) -> String {
    format!("{}.0.0.0/8", (ip_u32 >> 24) as u8)
}

pub fn get_hosting_ranges() -> Vec<CidrRange> {
    vec![
        CidrRange { name: "OVH".into(), start: ip(51,81,0,0), end: ip(51,81,255,255), mask: 16 },
        CidrRange { name: "OVH".into(), start: ip(54,36,0,0), end: ip(54,39,255,255), mask: 16 },
        CidrRange { name: "OVH".into(), start: ip(141,94,0,0), end: ip(141,95,255,255), mask: 16 },
        CidrRange { name: "OVH".into(), start: ip(145,239,0,0), end: ip(145,239,255,255), mask: 16 },
        CidrRange { name: "OVH".into(), start: ip(167,114,0,0), end: ip(167,114,255,255), mask: 16 },
        CidrRange { name: "OVH".into(), start: ip(188,165,192,0), end: ip(188,165,255,255), mask: 18 },
        CidrRange { name: "Hetzner".into(), start: ip(5,9,0,0), end: ip(5,9,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(49,12,0,0), end: ip(49,13,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(49,14,0,0), end: ip(49,14,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(65,21,0,0), end: ip(65,21,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(88,198,0,0), end: ip(88,198,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(95,217,0,0), end: ip(95,217,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(116,202,0,0), end: ip(116,203,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(136,243,0,0), end: ip(136,243,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(142,132,0,0), end: ip(142,133,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(144,76,0,0), end: ip(144,77,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(148,251,0,0), end: ip(148,251,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(157,90,0,0), end: ip(157,90,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(167,235,0,0), end: ip(167,235,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(168,119,0,0), end: ip(168,119,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(176,9,0,0), end: ip(176,9,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(176,26,0,0), end: ip(176,27,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(188,40,0,0), end: ip(188,40,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(195,201,0,0), end: ip(195,201,255,255), mask: 16 },
        CidrRange { name: "Hetzner".into(), start: ip(213,133,0,0), end: ip(213,133,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(159,89,0,0), end: ip(159,89,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(165,22,0,0), end: ip(165,22,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(137,184,0,0), end: ip(137,184,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(142,93,0,0), end: ip(142,93,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(157,230,0,0), end: ip(157,230,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(159,203,0,0), end: ip(159,203,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(161,35,0,0), end: ip(161,35,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(167,71,0,0), end: ip(167,71,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(167,99,0,0), end: ip(167,99,255,255), mask: 16 },
        CidrRange { name: "DigitalOcean".into(), start: ip(170,64,0,0), end: ip(170,65,255,255), mask: 16 },
        CidrRange { name: "Vultr".into(), start: ip(45,32,0,0), end: ip(45,63,255,255), mask: 16 },
        CidrRange { name: "Vultr".into(), start: ip(108,61,0,0), end: ip(108,61,255,255), mask: 16 },
        CidrRange { name: "Vultr".into(), start: ip(149,28,0,0), end: ip(149,28,255,255), mask: 16 },
        CidrRange { name: "Vultr".into(), start: ip(155,138,0,0), end: ip(155,138,255,255), mask: 16 },
        CidrRange { name: "Vultr".into(), start: ip(199,247,0,0), end: ip(199,247,255,255), mask: 16 },
        CidrRange { name: "Vultr".into(), start: ip(207,148,0,0), end: ip(207,148,255,255), mask: 16 },
        CidrRange { name: "Linode".into(), start: ip(23,92,16,0), end: ip(23,92,31,255), mask: 20 },
        CidrRange { name: "Linode".into(), start: ip(45,33,0,0), end: ip(45,33,255,255), mask: 16 },
        CidrRange { name: "Linode".into(), start: ip(45,56,0,0), end: ip(45,56,255,255), mask: 16 },
        CidrRange { name: "Linode".into(), start: ip(45,79,0,0), end: ip(45,79,255,255), mask: 16 },
        CidrRange { name: "Linode".into(), start: ip(45,118,0,0), end: ip(45,118,255,255), mask: 16 },
        CidrRange { name: "Linode".into(), start: ip(50,116,0,0), end: ip(50,116,255,255), mask: 16 },
        CidrRange { name: "Linode".into(), start: ip(72,14,176,0), end: ip(72,14,191,255), mask: 20 },
        CidrRange { name: "Linode".into(), start: ip(96,126,96,0), end: ip(96,126,127,255), mask: 19 },
        CidrRange { name: "Linode".into(), start: ip(139,144,0,0), end: ip(139,144,255,255), mask: 16 },
        CidrRange { name: "Linode".into(), start: ip(172,104,0,0), end: ip(172,105,255,255), mask: 16 },
        CidrRange { name: "Contabo".into(), start: ip(161,97,0,0), end: ip(161,97,255,255), mask: 16 },
        CidrRange { name: "Contabo".into(), start: ip(173,212,192,0), end: ip(173,212,255,255), mask: 18 },
        CidrRange { name: "Contabo".into(), start: ip(178,18,240,0), end: ip(178,18,255,255), mask: 20 },
        CidrRange { name: "Contabo".into(), start: ip(185,157,80,0), end: ip(185,157,95,255), mask: 20 },
        CidrRange { name: "AWS EC2".into(), start: ip(3,0,0,0), end: ip(3,255,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(13,32,0,0), end: ip(13,63,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(13,112,0,0), end: ip(13,127,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(15,177,0,0), end: ip(15,177,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(18,0,0,0), end: ip(18,255,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(35,0,0,0), end: ip(35,255,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(43,192,0,0), end: ip(43,199,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(44,192,0,0), end: ip(44,255,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(52,0,0,0), end: ip(52,95,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(54,144,0,0), end: ip(54,255,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(63,32,0,0), end: ip(63,35,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(64,252,64,0), end: ip(64,252,127,255), mask: 18 },
        CidrRange { name: "AWS EC2".into(), start: ip(75,2,0,0), end: ip(75,2,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(96,0,0,0), end: ip(96,0,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(99,77,0,0), end: ip(99,77,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(107,20,0,0), end: ip(107,23,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(140,179,0,0), end: ip(140,179,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(150,222,0,0), end: ip(150,222,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(157,175,0,0), end: ip(157,175,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(176,32,0,0), end: ip(176,35,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(184,72,0,0), end: ip(184,73,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(184,168,0,0), end: ip(184,169,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(185,48,0,0), end: ip(185,63,255,255), mask: 16 },
        CidrRange { name: "AWS EC2".into(), start: ip(204,236,128,0), end: ip(204,236,255,255), mask: 17 },
        CidrRange { name: "AWS EC2".into(), start: ip(216,182,224,0), end: ip(216,182,255,255), mask: 19 },
    ]
}

pub fn get_residential_ranges() -> Vec<CidrRange> {
    vec![
        CidrRange { name: "Comcast".into(), start: ip(73,15,0,0), end: ip(73,15,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(73,38,0,0), end: ip(73,38,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(73,42,0,0), end: ip(73,42,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(73,51,0,0), end: ip(73,51,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(73,103,0,0), end: ip(73,103,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(73,162,0,0), end: ip(73,162,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(73,179,0,0), end: ip(73,179,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(73,194,0,0), end: ip(73,194,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(73,240,0,0), end: ip(73,240,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(73,249,0,0), end: ip(73,249,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(98,0,0,0), end: ip(98,0,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(98,14,0,0), end: ip(98,14,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(98,28,0,0), end: ip(98,28,255,255), mask: 16 },
        CidrRange { name: "Comcast".into(), start: ip(98,30,0,0), end: ip(98,30,255,255), mask: 16 },
        CidrRange { name: "Verizon".into(), start: ip(100,0,0,0), end: ip(100,1,255,255), mask: 16 },
        CidrRange { name: "Verizon".into(), start: ip(100,36,0,0), end: ip(100,36,255,255), mask: 16 },
        CidrRange { name: "Verizon".into(), start: ip(100,38,0,0), end: ip(100,38,255,255), mask: 16 },
        CidrRange { name: "Verizon".into(), start: ip(108,0,0,0), end: ip(108,0,255,255), mask: 16 },
        CidrRange { name: "Verizon".into(), start: ip(108,10,0,0), end: ip(108,10,255,255), mask: 16 },
        CidrRange { name: "Verizon".into(), start: ip(108,13,0,0), end: ip(108,13,255,255), mask: 16 },
        CidrRange { name: "AT&T".into(), start: ip(12,180,0,0), end: ip(12,181,255,255), mask: 16 },
        CidrRange { name: "AT&T".into(), start: ip(12,190,0,0), end: ip(12,190,255,255), mask: 16 },
        CidrRange { name: "AT&T".into(), start: ip(99,0,0,0), end: ip(99,0,255,255), mask: 16 },
        CidrRange { name: "AT&T".into(), start: ip(99,4,0,0), end: ip(99,4,255,255), mask: 16 },
        CidrRange { name: "AT&T".into(), start: ip(99,6,0,0), end: ip(99,6,255,255), mask: 16 },
        CidrRange { name: "AT&T".into(), start: ip(99,8,0,0), end: ip(99,8,255,255), mask: 16 },
        CidrRange { name: "AT&T".into(), start: ip(99,12,0,0), end: ip(99,12,255,255), mask: 16 },
        CidrRange { name: "Spectrum".into(), start: ip(24,24,0,0), end: ip(24,24,255,255), mask: 16 },
        CidrRange { name: "Spectrum".into(), start: ip(24,61,0,0), end: ip(24,61,255,255), mask: 16 },
        CidrRange { name: "Spectrum".into(), start: ip(24,62,0,0), end: ip(24,62,255,255), mask: 16 },
        CidrRange { name: "Spectrum".into(), start: ip(24,63,0,0), end: ip(24,63,255,255), mask: 16 },
        CidrRange { name: "Spectrum".into(), start: ip(24,93,0,0), end: ip(24,93,255,255), mask: 16 },
        CidrRange { name: "Spectrum".into(), start: ip(24,98,0,0), end: ip(24,98,255,255), mask: 16 },
        CidrRange { name: "Spectrum".into(), start: ip(24,99,0,0), end: ip(24,99,255,255), mask: 16 },
        CidrRange { name: "DTAG".into(), start: ip(79,192,0,0), end: ip(79,255,255,255), mask: 16 },
        CidrRange { name: "DTAG".into(), start: ip(80,128,0,0), end: ip(80,159,255,255), mask: 16 },
        CidrRange { name: "Orange".into(), start: ip(90,0,0,0), end: ip(90,99,255,255), mask: 16 },
        CidrRange { name: "Vodafone".into(), start: ip(77,96,0,0), end: ip(77,111,255,255), mask: 16 },
        CidrRange { name: "British Telecom".into(), start: ip(81,128,0,0), end: ip(81,159,255,255), mask: 16 },
        CidrRange { name: "KPN".into(), start: ip(77,160,0,0), end: ip(77,169,255,255), mask: 16 },
    ]
}

fn ip(a: u8, b: u8, c: u8, d: u8) -> Ipv4Addr {
    Ipv4Addr::new(a, b, c, d)
}

pub fn ip_to_u32(ip: Ipv4Addr) -> u32 {
    let octets = ip.octets();
    (octets[0] as u32) << 24 | (octets[1] as u32) << 16 | (octets[2] as u32) << 8 | octets[3] as u32
}

pub fn u32_to_ip(val: u32) -> Ipv4Addr {
    Ipv4Addr::new(
        (val >> 24) as u8,
        (val >> 16) as u8,
        (val >> 8) as u8,
        val as u8,
    )
}
