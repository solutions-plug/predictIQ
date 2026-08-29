/**
 * Manual Accessibility Testing Checklist
 *
 * Automated tools (axe-core, Lighthouse, jest-axe) catch most WCAG 2.1 AA
 * violations, but several categories require a human to actually try the
 * page. This prints that checklist so `npm run a11y:manual` has something
 * to run — it previously pointed at a script that didn't exist in this repo.
 */

const CHECKLIST = [
  {
    category: 'Keyboard navigation',
    items: [
      'Tab through the entire page — every interactive element is reachable and in a logical order',
      'Focus is always visible (no invisible focus outlines)',
      'No keyboard trap: focus can always move forward and backward out of any widget',
      'Modals/dialogs trap focus while open and return it to the trigger on close',
      'Skip-to-content link is the first focusable element and actually works',
    ],
  },
  {
    category: 'Screen reader (VoiceOver / NVDA / JAWS)',
    items: [
      'Page landmarks (banner, main, navigation, contentinfo) are announced correctly',
      'Headings form a logical, non-skipping hierarchy',
      'Form fields announce their label, required state, and any validation error',
      'Live regions (toasts, status updates) are announced without moving focus',
      'Images convey their alt text (or are correctly marked decorative)',
    ],
  },
  {
    category: 'Color & contrast',
    items: [
      'Text meets 4.5:1 contrast (3:1 for large text) in both light and dark mode',
      'Information is never conveyed by color alone (e.g. status badges also use text/icons)',
      'Focus indicators meet 3:1 contrast against their background',
    ],
  },
  {
    category: 'Zoom & reflow',
    items: [
      'Page is usable at 200% browser zoom with no horizontal scroll or clipped content',
      'Page is usable at 400% zoom in a 1280px viewport (WCAG 1.4.10 reflow)',
    ],
  },
  {
    category: 'Motion & animation',
    items: [
      'prefers-reduced-motion is respected for any animated transitions',
      'No content flashes more than 3 times per second',
    ],
  },
];

function printChecklist() {
  console.log('Manual Accessibility Testing Checklist (WCAG 2.1 AA)\n');
  for (const { category, items } of CHECKLIST) {
    console.log(category);
    for (const item of items) {
      console.log(`  [ ] ${item}`);
    }
    console.log('');
  }
  console.log('Run alongside `npm run test:a11y`, `npm run lighthouse`, and `npm run axe`.');
}

printChecklist();
