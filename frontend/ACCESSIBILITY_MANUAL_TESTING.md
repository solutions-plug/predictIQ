# Manual Accessibility Testing Checklist

## Overview
This document provides a comprehensive manual testing checklist for WCAG 2.1 AA compliance. Perform these tests in addition to automated testing to ensure complete accessibility coverage.

## Testing Environment

### Required Tools
- **Windows**: NVDA (free), JAWS (trial/licensed)
- **macOS**: VoiceOver (built-in)
- **iOS**: VoiceOver (built-in)
- **Browsers**: Chrome, Firefox, Safari, Edge
- **Extensions**: 
  - axe DevTools
  - WAVE
  - Lighthouse
  - Color Contrast Analyzer

## Screen Reader Testing

### NVDA (Windows)

#### Setup
1. Download NVDA from https://www.nvaccess.org/
2. Install and restart computer
3. Launch NVDA (Ctrl + Alt + N)
4. Open browser and navigate to landing page

#### Test Checklist

- [ ] **Page Load**
  - [ ] Page title is announced
  - [ ] Main heading is announced
  - [ ] Landmark regions are identified

- [ ] **Navigation**
  - [ ] Skip link works (press Enter on "Skip to main content")
  - [ ] All navigation links are announced with meaningful text
  - [ ] Current page/section is indicated
  - [ ] Tab order is logical

- [ ] **Headings**
  - [ ] Navigate by headings (H key)
  - [ ] Heading hierarchy is correct (H1 → H2 → H3)
  - [ ] All headings are meaningful

- [ ] **Forms**
  - [ ] Form labels are announced
  - [ ] Required fields are indicated
  - [ ] Error messages are announced
  - [ ] Success messages are announced
  - [ ] Field instructions are read

- [ ] **Images**
  - [ ] Logo alt text is read
  - [ ] Decorative images are skipped
  - [ ] Informative images have descriptive alt text

- [ ] **Links**
  - [ ] All links have meaningful text
  - [ ] Link purpose is clear from context
  - [ ] External links are indicated

- [ ] **Buttons**
  - [ ] Button labels are descriptive
  - [ ] Button state changes are announced
  - [ ] Disabled state is announced

- [ ] **Dynamic Content**
  - [ ] Form submission success is announced
  - [ ] Error messages appear in aria-live region
  - [ ] Loading states are announced

#### NVDA Commands Reference
- `Insert + Down Arrow`: Read all
- `H`: Next heading
- `Shift + H`: Previous heading
- `K`: Next link
- `B`: Next button
- `F`: Next form field
- `Insert + F7`: Elements list

### JAWS (Windows)

#### Setup
1. Download JAWS from https://www.freedomscientific.com/
2. Install (trial or licensed version)
3. Launch JAWS
4. Open browser and navigate to landing page

#### Test Checklist

- [ ] **Page Load**
  - [ ] Page title is announced
  - [ ] Number of headings/links/forms announced
  - [ ] Main content is identified

- [ ] **Navigation**
  - [ ] Skip link functions correctly
  - [ ] Landmarks are navigable (R key)
  - [ ] Tab order is logical
  - [ ] Focus is visible

- [ ] **Forms**
  - [ ] Form mode activates automatically
  - [ ] Labels are associated correctly
  - [ ] Error messages are announced immediately
  - [ ] Required fields are indicated

- [ ] **Tables** (if applicable)
  - [ ] Table headers are announced
  - [ ] Cell relationships are clear

- [ ] **Lists**
  - [ ] Lists are identified
  - [ ] List item count is announced
  - [ ] Nested lists are handled correctly

#### JAWS Commands Reference
- `Insert + F5`: Form fields list
- `Insert + F6`: Headings list
- `Insert + F7`: Links list
- `R`: Next region/landmark
- `H`: Next heading
- `T`: Next table

### VoiceOver (macOS)

#### Setup
1. Enable VoiceOver: Cmd + F5
2. Open Safari or Chrome
3. Navigate to landing page

#### Test Checklist

- [ ] **Page Load**
  - [ ] Page title is announced
  - [ ] Main heading is announced
  - [ ] Landmark regions are identified

- [ ] **Navigation**
  - [ ] Rotor navigation works (Cmd + U)
  - [ ] Skip link is accessible
  - [ ] Tab order is logical
  - [ ] Focus indicator is visible

- [ ] **Forms**
  - [ ] Form controls are announced with labels
  - [ ] Required fields are indicated
  - [ ] Error messages are announced
  - [ ] Validation feedback is clear

- [ ] **Gestures** (if testing on iOS)
  - [ ] Swipe right/left navigates elements
  - [ ] Double-tap activates elements
  - [ ] Rotor gestures work

#### VoiceOver Commands Reference
- `Cmd + F5`: Toggle VoiceOver
- `Ctrl + Option + Right Arrow`: Next item
- `Ctrl + Option + Left Arrow`: Previous item
- `Ctrl + Option + U`: Rotor
- `Ctrl + Option + Cmd + H`: Next heading

### VoiceOver (iOS)

#### Setup
1. Settings → Accessibility → VoiceOver → On
2. Open Safari
3. Navigate to landing page

#### Test Checklist

- [ ] **Touch Navigation**
  - [ ] Swipe right/left navigates elements
  - [ ] Double-tap activates elements
  - [ ] Elements are announced clearly

- [ ] **Rotor**
  - [ ] Rotate two fingers to access rotor
  - [ ] Navigate by headings
  - [ ] Navigate by links
  - [ ] Navigate by form controls

- [ ] **Forms**
  - [ ] Form fields are accessible
  - [ ] Keyboard appears for text input
  - [ ] Error messages are announced
  - [ ] Submit button is accessible

## Keyboard Navigation Testing

### Test Checklist

- [ ] **Tab Order**
  - [ ] Tab moves forward through interactive elements
  - [ ] Shift + Tab moves backward
  - [ ] Order is logical and intuitive
  - [ ] No keyboard traps

- [ ] **Focus Indicators**
  - [ ] Focus is always visible
  - [ ] Focus indicator has sufficient contrast (3:1)
  - [ ] Focus indicator is not obscured

- [ ] **Interactive Elements**
  - [ ] Links activate with Enter
  - [ ] Buttons activate with Enter or Space
  - [ ] Form fields are accessible
  - [ ] Dropdowns work with arrow keys

- [ ] **Skip Links**
  - [ ] Skip link appears on first Tab
  - [ ] Skip link moves focus to main content
  - [ ] Skip link is visible when focused

- [ ] **No Mouse Required**
  - [ ] All functionality available via keyboard
  - [ ] No hover-only content
  - [ ] No click-only interactions

## Color Contrast Testing

### Tools
- Chrome DevTools (Inspect → Accessibility)
- WebAIM Contrast Checker
- Color Contrast Analyzer (CCA)

### Test Checklist

- [ ] **Text Contrast**
  - [ ] Normal text: 4.5:1 minimum
  - [ ] Large text (18pt+): 3:1 minimum
  - [ ] Test all text colors against backgrounds

- [ ] **UI Components**
  - [ ] Buttons: 3:1 contrast
  - [ ] Form borders: 3:1 contrast
  - [ ] Focus indicators: 3:1 contrast
  - [ ] Icons: 3:1 contrast

- [ ] **States**
  - [ ] Hover states have sufficient contrast
  - [ ] Focus states have sufficient contrast
  - [ ] Disabled states are distinguishable
  - [ ] Error states have sufficient contrast

- [ ] **Color Independence**
  - [ ] Information not conveyed by color alone
  - [ ] Error messages use icons + text
  - [ ] Required fields use * + label

## Zoom and Reflow Testing

### Test Checklist

- [ ] **Browser Zoom (200%)**
  - [ ] No horizontal scrolling
  - [ ] All content is readable
  - [ ] No content is cut off
  - [ ] Functionality remains intact

- [ ] **Text Resize (200%)**
  - [ ] Text scales properly
  - [ ] No text overlap
  - [ ] Layout adapts appropriately

- [ ] **Mobile Viewport**
  - [ ] Content reflows correctly
  - [ ] Touch targets are 44x44px minimum
  - [ ] No horizontal scrolling

## Form Testing

### Test Checklist

- [ ] **Labels**
  - [ ] All inputs have visible labels
  - [ ] Labels are programmatically associated
  - [ ] Placeholder text is not used as labels

- [ ] **Required Fields**
  - [ ] Required fields are indicated visually
  - [ ] Required fields have aria-required="true"
  - [ ] Asterisk has aria-label="required"

- [ ] **Error Handling**
  - [ ] Errors are announced to screen readers
  - [ ] Error messages are specific and helpful
  - [ ] Errors are associated with fields (aria-describedby)
  - [ ] Focus moves to first error

- [ ] **Success Messages**
  - [ ] Success is announced to screen readers
  - [ ] Success message is visible
  - [ ] Success message uses aria-live region

## Content Testing

### Test Checklist

- [ ] **Headings**
  - [ ] One H1 per page
  - [ ] Heading hierarchy is logical
  - [ ] Headings describe content
  - [ ] No skipped heading levels

- [ ] **Links**
  - [ ] Link text is descriptive
  - [ ] No "click here" or "read more" without context
  - [ ] External links are indicated
  - [ ] Links are distinguishable from text

- [ ] **Images**
  - [ ] All images have alt attributes
  - [ ] Alt text is descriptive
  - [ ] Decorative images have alt=""
  - [ ] Complex images have long descriptions

- [ ] **Language**
  - [ ] Page language is set (lang attribute)
  - [ ] Language changes are marked
  - [ ] Content is clear and concise

## Responsive Design Testing

### Test Checklist

- [ ] **Mobile (320px - 767px)**
  - [ ] Content is readable
  - [ ] Touch targets are adequate
  - [ ] No horizontal scrolling
  - [ ] Navigation is accessible

- [ ] **Tablet (768px - 1023px)**
  - [ ] Layout adapts appropriately
  - [ ] All features are accessible
  - [ ] Touch targets are adequate

- [ ] **Desktop (1024px+)**
  - [ ] Layout is optimal
  - [ ] All features are accessible
  - [ ] Keyboard navigation works

## Testing Results Template

### Test Session Information
- **Date**: [Date]
- **Tester**: [Name]
- **Environment**: [OS, Browser, Screen Reader]
- **Page Tested**: [URL]

### Results Summary
- **Total Tests**: [Number]
- **Passed**: [Number]
- **Failed**: [Number]
- **Not Applicable**: [Number]

### Issues Found

#### Issue 1
- **Severity**: [Critical/Serious/Moderate/Minor]
- **WCAG Criterion**: [e.g., 1.3.1 Info and Relationships]
- **Description**: [Detailed description]
- **Steps to Reproduce**: [Steps]
- **Expected Behavior**: [What should happen]
- **Actual Behavior**: [What actually happens]
- **Screenshot**: [If applicable]
- **Recommendation**: [How to fix]

### Sign-off
- [ ] All critical issues resolved
- [ ] All serious issues resolved
- [ ] Moderate issues documented
- [ ] Minor issues documented
- [ ] Ready for production

**Tester Signature**: _______________
**Date**: _______________

## Resources

### WCAG 2.1 Guidelines
- https://www.w3.org/WAI/WCAG21/quickref/

### Screen Reader Resources
- NVDA User Guide: https://www.nvaccess.org/files/nvda/documentation/userGuide.html
- JAWS Documentation: https://www.freedomscientific.com/training/jaws/
- VoiceOver Guide: https://support.apple.com/guide/voiceover/welcome/mac

### Testing Tools
- axe DevTools: https://www.deque.com/axe/devtools/
- WAVE: https://wave.webaim.org/
- Color Contrast Analyzer: https://www.tpgi.com/color-contrast-checker/

### Best Practices
- WebAIM: https://webaim.org/
- A11y Project: https://www.a11yproject.com/
- Inclusive Components: https://inclusive-components.design/
