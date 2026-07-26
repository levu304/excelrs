## ADDED Requirements

### Requirement: BorderStyleStyle covers all 12 OOXML §18.18.3 styles

The `BorderStyleStyle` enum SHALL include all 12 OOXML border line styles: `None`, `Thin`, `Medium`, `Thick`, `Dashed`, `Dotted`, `Double`, `Hair`, `DashDot`, `DashDotDot`, `MediumDashDot`, `SlantDashDot`, `MediumDashed`, `MediumDashDotDot`.

#### Scenario: Hair style round-trips

- **WHEN** parsing `<left style="hair"/>`
- **THEN** `left.style` SHALL be `BorderStyleStyle::Hair`
- **WHEN** emitting `BorderStyleStyle::Hair`
- **THEN** the XML attribute SHALL be `style="hair"`

#### Scenario: DashDot style round-trips

- **WHEN** parsing `<right style="dashDot"/>`
- **THEN** `right.style` SHALL be `BorderStyleStyle::DashDot`
- **WHEN** emitting `BorderStyleStyle::DashDot`
- **THEN** the XML attribute SHALL be `style="dashDot"`

#### Scenario: DashDotDot style round-trips

- **WHEN** parsing `<left style="dashDotDot"/>`
- **THEN** `left.style` SHALL be `BorderStyleStyle::DashDotDot`

#### Scenario: MediumDashDot style round-trips

- **WHEN** parsing `<right style="mediumDashDot"/>`
- **THEN** `right.style` SHALL be `BorderStyleStyle::MediumDashDot`

#### Scenario: SlantDashDot style round-trips

- **WHEN** parsing `<top style="slantDashDot"/>`
- **THEN** `top.style` SHALL be `BorderStyleStyle::SlantDashDot`

#### Scenario: MediumDashed style round-trips

- **WHEN** parsing `<bottom style="mediumDashed"/>`
- **THEN** `bottom.style` SHALL be `BorderStyleStyle::MediumDashed`

#### Scenario: MediumDashDotDot style round-trips

- **WHEN** parsing `<diagonal style="mediumDashDotDot"/>`
- **THEN** `diagonal.style` SHALL be `BorderStyleStyle::MediumDashDotDot`

### Requirement: From<&str> is case-insensitive for border styles

The `From<&str>` impl for `BorderStyleStyle` SHALL accept case-insensitive input (e.g. `"Hair"`, `"dashDot"`, `"DASHDOT"`).

#### Scenario: Case-insensitive parse

- **WHEN** calling `BorderStyleStyle::from("HAIR")`
- **THEN** the result SHALL be `BorderStyleStyle::Hair`
- **WHEN** calling `BorderStyleStyle::from("DASHDOT")`
- **THEN** the result SHALL be `BorderStyleStyle::DashDot`
