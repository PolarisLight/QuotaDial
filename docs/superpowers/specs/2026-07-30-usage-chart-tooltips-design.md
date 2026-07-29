# Usage Chart Tooltip Design

## Goal

Make the combined Token bar chart and remaining-quota line chart inspectable without adding permanent labels or repeating information already visible on the chart.

## Interaction

- Hovering or focusing a Token bar opens a floating tooltip near that bar.
- The Token tooltip contains only the exact localized Token count, such as `530,000`.
- Hovering or focusing a remaining-quota point opens the same tooltip style near that point.
- The quota tooltip contains only the exact remaining percentage with one decimal place, such as `82.0%`.
- Dates, series names, and other information already visible in the chart are not repeated.
- The active bar or point receives a subtle highlight. The rest of the chart remains unchanged.
- The tooltip disappears when the pointer leaves the target or keyboard focus moves away.
- On pointer devices without hover, tapping a target opens its tooltip and tapping elsewhere closes it.

## Placement and Visual Style

- Use one compact macOS-style floating surface shared by both series.
- Place it above the active target by default.
- Shift the tooltip inward when the target is close to the left or right chart boundary.
- Keep the tooltip inside the chart container and above chart marks.
- Use the existing dark/light theme variables, a thin border, small radius, and restrained shadow.
- Do not add a persistent readout, crosshair, or duplicate legend.

## Component Design

`UsageQuotaChart` owns a single active-tooltip state:

- series: `tokens` or `quota`
- formatted value
- anchor coordinates

Each SVG Token bar and quota point is an interaction target with:

- pointer enter/leave handling
- focus/blur handling
- tap/click handling
- an accessible label containing the full semantic value
- keyboard focusability

The tooltip is rendered once as an HTML overlay inside the chart wrapper. SVG target coordinates are converted to percentage-based overlay coordinates so the tooltip remains aligned while the chart scales.

## Data and Formatting

- Token values use the existing Chinese locale number formatter without compact notation.
- Remaining quota uses one fractional digit followed by `%`.
- No new backend fields or persistence are required.

## Accessibility

- Bars and quota points are keyboard focusable.
- Focus produces the same visible tooltip as hover.
- Each target exposes a complete accessible label even though the visible tooltip deliberately omits redundant context.
- The tooltip itself is presentational and does not produce duplicate screen-reader announcements.
- Focus indicators remain visible against both themes.

## Tests

Component tests will verify:

- hovering a bar shows the exact Token count
- leaving the bar hides the tooltip
- focusing a bar provides the same tooltip
- hovering or focusing a quota point shows one-decimal remaining percentage
- tooltip text does not include the date or series name
- activating a different target replaces the existing tooltip
- keyboard-accessible labels remain complete

Browser verification will cover tooltip placement near chart edges, dark-theme contrast, bar/point highlighting, and preservation of the existing chart layout.

## Out of Scope

- Zooming, panning, brushing, or selecting a date range
- A persistent crosshair
- Comparing multiple dates in one tooltip
- Repeating dates or series names inside the tooltip
- Changes to quota or Token collection
