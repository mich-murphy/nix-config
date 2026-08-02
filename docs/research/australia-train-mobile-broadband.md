# Dedicated LTE/5G connectivity for Australian train commuting

Research checked 30 July 2026. Prices are Australian dollars unless stated otherwise.

## Recommendation

The best premium, still-portable setup is:

1. Test Telstra, Optus, and Vodafone on the exact train route before buying hardware.
2. If Telstra wins, buy the **Telstra-certified NETGEAR Nighthawk M7 Ultra MR7500**.
3. Place it directly beside the window and USB-tether it to the laptop.
4. Add NETGEAR's **$109 passive MIMO window antenna** only if the modem regularly shows fewer than two bars.
5. Keep the phone on whichever of Optus or Vodafone best covers Telstra's weak sections.

The M7 Ultra is the strongest practical choice because it combines a current Qualcomm X75 radio; Australian low-band LTE B28 and 5G n26/n28; 5G n78; a removable 5,185 mAh battery; a 2.5 GbE port; and, most importantly, two TS-9 external cellular-antenna ports in a 245–275 g battery device. It is certified for Telstra, although Telstra marks it **not Blue Tick**, so there is no carrier claim that it has enhanced weak-signal antenna sensitivity. The current Telstra device cost is $898.92 over 36 months plus the selected plan. [NETGEAR technical specification](https://www.downloads.netgear.com/files/GDC/MR7500/MR7500_TS.pdf), [Telstra specifications and price](https://www.telstra.com.au/internet/mobile-broadband/netgear/nighthawk-m7-ultra), [Telstra mobile-broadband device list](https://www.telstra.com.au/internet/mobile-broadband)

This recommendation is conditional. A hotspot uses the same cellular technology as a phone and is subject to the same propagation constraints; NETGEAR itself recommends window placement and identifies metal as a signal obstacle. The gain comes mainly from a dedicated radio that can stay in the best position, optional external antennas, independent battery and carrier choice—not from Wi-Fi 7. [NETGEAR placement guidance](https://kb.netgear.com/000065270/Where-should-I-place-my-NETGEAR-mobile-hotspot)

## Why the train is the hard part

Australian carrier coverage maps predict **outdoor** coverage. Telstra says vehicle bodies can reduce or block signal, movement causes tower handovers and fluctuating quality, and crowds can slow a network even where signal is otherwise good. Its practical advice for a passenger is to keep the device near a window. [Telstra coverage-map limitations and placement guidance](https://www.telstra.com.au/coverage-networks/our-coverage)

That creates three different failure modes:

- **Carriage attenuation:** a window-positioned hotspot or passive window antenna can help a weak-but-usable signal.
- **Rail-corridor dead zone or tunnel:** no modem can create a network signal that is not present.
- **Peak-hour congestion:** stronger reception does not add capacity to an overloaded cell.

A professional vehicle router normally earns its price through roof-mounted antennas, permanent power, and multiple cellular links. A passenger cannot put an antenna outside a public train, so most of that advantage disappears inside the same metal carriage.

## Australian network requirements

Do not buy an LTE-only product for this use. Buy a 5G modem with strong LTE fallback:

- **LTE Band 28 (700 MHz)** is essential. Telstra calls B28 its main 4G coverage frequency and warns that devices without it may lose service where it is the only available 4G layer. Optus gives the same B28 compatibility warning. [Telstra 3G-closure guidance](https://www.telstra.com.au/support/mobiles-devices/3g-closure), [Optus compatibility guidance](https://www.optus.com.au/prepaid/sim-plans)
- For a broadly useful Australian device, look for LTE B1/B3/B7/B28 and 5G n28/n78, with n5/n7/n26 also valuable on Telstra. Telstra's M7 Ultra configuration covers LTE B1/B3/B7/B28 and 5G n5/n7/n26/n28/n78. [Telstra M7 Ultra specifications](https://www.telstra.com.au/internet/mobile-broadband/netgear/nighthawk-m7-ultra)
- Australia's 3G networks have closed, so 3G support is not a fallback strategy. Telstra reports 4G population coverage of 99.7% and 5G above 93%; Optus reports 98.5% combined coverage and 84.65% 5G population coverage; Vodafone reports 98.4% population coverage after its 2025 regional expansion. These nationwide figures do **not** establish performance on one railway line. [Telstra coverage figures](https://www.telstra.com.au/support/mobiles-devices/3g-closure), [Optus FY25 Sustainability Report](https://www.optus.com.au/content/dam/optus/documents/about-us/sustainability/reporting/2025/optus-fy25-sustainability-report.pdf), [Vodafone network expansion](https://www.vodafone.com.au/network/coverage-expansion)
- Since 30 June 2026, the three network operators have had to publish standardised maps with good, moderate, basic, and no-coverage categories. Those maps remain predictive, so a real commute trial is still decisive. [ACMA coverage-map rules](https://www.acma.gov.au/articles/2026-03/new-rules-mobile-phone-coverage-maps)

## Ranked portable shortlist

| Rank | Device | Mobile and antenna capability | Portable practicality | Verdict and current official price |
| --- | --- | --- | --- | --- |
| 1 | **NETGEAR Nighthawk M7 Ultra MR7500, Telstra** | X75; LTE B1/B3/B7/B28; 5G n5/n7/n26/n28/n78/n258; two TS-9 ports; single nano-SIM; no eSIM | Removable 5,185 mAh battery; about 245–275 g; USB-C; 2.5 GbE; Wi-Fi 7; up to 64 clients | Best train choice because it is local, carrier-certified, and antenna-expandable. Not Blue Tick and not dual-SIM. **$898.92** over 36 months plus plan. [NETGEAR](https://www.downloads.netgear.com/files/GDC/MR7500/MR7500_TS.pdf), [Telstra](https://www.telstra.com.au/internet/mobile-broadband/netgear/nighthawk-m7-ultra) |
| 2 | **Sonim H700, Telstra** | X75; Telstra lists LTE B1/B3/B7/B26/B28 with 4×4 MIMO and 5G n5/n7/n26/n78/n258; two TS-9 ports; 2.5 GbE; single nano-SIM | Removable 6,000 mAh battery; up to 14 hours; 308 g; IP68 and MIL-STD-810H; Wi-Fi 6E certified and Wi-Fi 7 ready | Best-value premium alternative: a newer radio, longer stated battery life, rugged body, and external antennas at far lower cost. Also not Blue Tick and less pocketable. **$518.76** over 36 months plus plan. [Sonim](https://www.sonimtech.com/products/hotspots/h700-mobile-hotspot), [Telstra](https://www.telstra.com.au/internet/mobile-broadband/sonim/h700-5g-hotspot), [Telstra device list](https://www.telstra.com.au/internet/mobile-broadband) |
| 3 | **NETGEAR Nighthawk M7 MH7150, unlocked** | Broad LTE B1/B3/B5/B7/B28 and 5G n26/n28/n78 support, plus physical SIM and eSIM | Non-removable 3,850 mAh battery; up to 10 hours; 240 g; USB-C; Wi-Fi 7; Ethernet needs a separate adapter or cradle | Best if route testing favours Optus or Vodafone, or carrier flexibility matters more than antennas. The published port list has no TS-9 antenna ports, and SIM plus eSIM is not documented as simultaneous carrier failover. **$799 direct.** [NETGEAR Australia](https://www.netgear.com/au/mobile-wifi/hotspots/mh7150/) |
| 4 | **GL.iNet Puli AX GL-XE3000** | Broad global LTE/5G bands including B28 and n26/n28/n78; four SMA cellular antennas; two nano-SIM slots, but only one SIM active at a time | Built-in 47.4 Wh battery; 761 g; six protruding antennas; 2.5 GbE plus 1 GbE; Wi-Fi 6; OpenWrt | Most capable self-powered dual-SIM enthusiast option, but too bulky for most commutes and switching SIMs is not dual-active bonding. The manufacturer page does not state Australian RCM or carrier certification; verify the exact model, RCM, TAC, and IMEI with the intended carrier before importing. [GL.iNet product](https://www.gl-inet.com/products/gl-xe3000/), [GL.iNet dual-SIM documentation](https://docs.gl-inet.com/router/en/4/user_guide/gl-xe3000/), [ACMA device rules](https://www.acma.gov.au/cellular-mobile-telecom-devices-class-licence) |
| 5 | **Telstra 5G Hotspot, ZTE MU5120** | X62; LTE B1/B3/B5/B7/B8/B26/B28 and 5G n1/n3/n5/n7/n8/n28/n41/n77/n78; single SIM; no external antenna ports listed | 10,000 mAh battery; claimed up to 16 hours; 240 g; Wi-Fi 6; up to 64 clients | Battery-life pick, not the weak-signal pick. It is inexpensive and locally supported, but lacks the M7 Ultra's external-antenna path. **$318.96** over 36 months plus plan. [Telstra product](https://www.telstra.com.au/internet/mobile-broadband/telstra/telstra-5g-hotspot), [Telstra device list](https://www.telstra.com.au/internet/mobile-broadband) |

The unlocked M7's band list is well suited to Australia, but Optus says a data SIM may be used only in a compatible 4G/5G device and directs customers to its IMEI checker. A matching band list is therefore not a substitute for carrier acceptance of the exact device. [Optus Data SIM compatibility](https://www.optus.com.au/broadband-nbn/mobile-broadband/sim-only-data-plans/shop)

## Passive antenna recommendation

NETGEAR's 6000451 is a small 2×2 MIMO antenna covering 600–960 MHz and 1,710–5,925 MHz, with up to 2.5 dBi gain, suction cups, clips, two SMA plugs, and two TS-9 adapters. NETGEAR Australia sells it for **$109**. [NETGEAR antenna specification and price](https://www.netgear.com/au/mobile-wifi/hotspots/omnidirectional-mimo-antenna/)

Use it as a measured experiment, not an automatic upgrade:

- First test the hotspot itself against the window.
- If the device regularly shows fewer than two bars, suction the passive antenna to the window and compare.
- NETGEAR says an external antenna can improve reception and transfer speeds below two bars, but may reduce reception through interference when the modem already has more than two bars.
- Move it and retest; do not assume that a higher bar count always means lower packet loss.
- Treat the TS-9 connectors gently because NETGEAR describes them as delicate.

[NETGEAR external-antenna guidance](https://kb.netgear.com/000065791/How-do-I-improve-the-reception-of-my-NETGEAR-mobile-hotspot-with-an-external-antenna)

The antenna is still inside the carriage, so it cannot reproduce a roof-mounted vehicle antenna. Its realistic advantages are window placement, separation from the laptop and the user's body, and a repeatable antenna orientation.

## Carrier and SIM strategy

Carrier choice is likely to produce a bigger gain than moving from one current premium hotspot to another:

1. Obtain short-term Telstra, Optus, and Vodafone services.
2. Test the same morning and evening trains for several days.
3. Keep the device in the same window position and compare usable work outcomes: VPN continuity, call stability, upload success, and outage duration—not only a one-off speed test.
4. Buy the modem only after identifying the best primary carrier.

For diversity, **Telstra plus either Optus or Vodafone** is the safer general pairing. This is an inference from network ownership: the ACCC reports that Optus and TPG/Vodafone use shared Optus radio infrastructure across the regional multi-operator core-network area, while metropolitan radio networks and core networks remain separate. Consequently, Optus plus Vodafone may share a physical radio failure or coverage gap on some regional stretches. [ACCC Mobile Infrastructure Report 2025](https://www.accc.gov.au/by-industry/telecommunications-and-internet/mobile-services-regulation/mobile-infrastructure-report/mobile-infrastructure-report-2025)

Consumer hotspots are not true dual-carrier systems:

- The M7 Ultra, H700, and Telstra/ZTE hotspot are single-SIM.
- The unlocked M7 offers physical SIM and eSIM selection, not documented simultaneous links.
- The Puli AX holds two SIMs but its manual says only one is active; the second is standby.

If continuity is business-critical and employer policy permits it, a Mac can connect to the hotspot over Wi-Fi or Ethernet while using a phone from a second carrier over USB tethering. Speedify says its macOS client can bond those independent connections; because Speedify is itself an internet VPN rather than a corporate VPN, obtain IT approval and test compatibility with the employer's VPN and security controls before relying on it. [Speedify macOS connection support](https://support.speedify.com/article/434-internet-connections-mac), [Speedify and corporate VPNs](https://support.speedify.com/article/762-speedify-vs-company-vpn)

## Professional and vehicle-class hardware

### Peplink MAX BR2 Pro

The BR2 Pro is the genuine dual-carrier option: two independent 5G modems, four SIM slots, eight cellular antenna connections, Wi-Fi 6, multiple WANs, and SpeedFusion failover, smoothing, and bonding. It supports Australian-useful B28 and n28/n78 in its global configuration. [Peplink BR2 Pro product page](https://www.peplink.com/products/mobile-routers/max-br2-pro/)

It is not a sensible everyday passenger device. It is about 1.1 kg before its ten supplied antennas and power supply, has no battery, and draws up to 30 W. Its advantages assume a powered installation and properly placed external antennas. [Peplink BR2 Pro datasheet](https://download.peplink.com/resources/peplink_br2_pro_datasheet.pdf)

### GL.iNet Spitz AX and Teltonika RUTX50

These are single-modem, dual-SIM routers rather than two live cellular links:

- The **GL.iNet Spitz AX GL-X3000** weighs 520 g, exposes four cellular and two Wi-Fi antennas, and requires 9–36 V DC at up to 14 W. It offers dual-SIM standby/failover, 2.5 GbE, Wi-Fi 6, and Australian-useful B28/n28/n78 bands. [GL.iNet product specification](https://www.gl-inet.com/products/gl-x3000/), [GL.iNet manual](https://docs.gl-inet.com/router/en/4/user_guide/gl-x3000/)
- The **Teltonika RUTX50** weighs 533 g, requires 9–50 V DC, draws up to 16.5 W, has four SMA cellular antennas and five Gigabit Ethernet ports, and supports dual-SIM auto-switching on weak signal, data limits, roaming, or connection failure. Its Wi-Fi is only Wi-Fi 5, which is nevertheless ample relative to a marginal cellular link. [Teltonika product specification](https://teltonika-networks.com/products/routers/rutx50), [Teltonika datasheet](https://teltonika-networks.com/cdn/products/2023/01/63b7f88131a497-19366814/datasheets/707278-rutx50-datasheet-2024-v18.pdf)

Both can be powered from a suitable battery system, but they create a bulky collection of router, antennas, cables, and power hardware while leaving every cellular antenna inside the train. They are better for a car, caravan, worksite, or fixed fringe-coverage installation.

## What not to buy

- Do not buy an LTE-only modem: it gives up current 5G coverage and longevity without solving carriage attenuation.
- Do not import a modem based only on a band list. ACMA says cellular data devices must comply with Australian technical standards, and a permit is needed for non-compliant equipment. Confirm RCM and carrier acceptance first. [ACMA cellular-device rules](https://www.acma.gov.au/cellular-mobile-telecom-devices-class-licence)
- Do not buy an online active “signal booster.” ACMA says mobile boosters are prohibited and cellular repeaters require written carrier permission, even for an exempt model. [ACMA repeater and booster rules](https://www.acma.gov.au/cellular-mobile-repeaters)
- Do not pay extra for mmWave, Wi-Fi 7, or multi-gigabit Ethernet expecting those features to improve a weak cellular signal. The relevant purchasing features are carrier compatibility, low-band support, radio generation, antenna ports, battery, and placement.

## Purchase and setup sequence

1. Use the carriers' standardised maps to identify obvious no-coverage sections, then perform live trials because the maps model outdoor use.
2. If the current phone is not on Telstra, test Telstra first. If it is already on Telstra, test Optus and Vodafone before assuming a Telstra hotspot will fix the problem.
3. If Telstra is best, choose:
   - **M7 Ultra** for the most polished, lightest antenna-expandable premium device.
   - **Sonim H700** for materially lower cost, larger battery, ruggedness, and the same X75 modem generation.
4. Set the hotspot at the window, keep it out of the bag, and USB-tether it to the laptop where convenient.
5. Record failures for a week before adding the passive antenna.
6. Add the NETGEAR MIMO antenna only for persistent sub-two-bar sections and keep it only if repeat testing improves usable work.
7. Keep the phone on a different physical network for fallback.
8. For unavoidable dead zones, make work resilient: sync repositories and documents before departure, queue uploads, and avoid scheduling critical video calls across known gaps.

## Final assessment

The premium spend can make the commute **more usable**, but not continuously connected in every carriage and tunnel. The most defensible purchase is the **M7 Ultra after a Telstra route trial**, with the **Sonim H700 as the better-value premium alternative**. The decisive accessory is not a faster Wi-Fi standard; it is an optional passive antenna placed at the window. If two carriers have complementary gaps, using the Telstra modem alongside an Optus or Vodafone phone is more likely to improve continuity than carrying an industrial router whose antennas remain trapped inside the same train.
