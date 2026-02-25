# Content Management & Update Guide

**PredictIQ Documentation**  
**Version:** 1.0  
**Last Updated:** February 2026  
**Audience:** Non-technical team members, content managers, marketing staff

---

## Table of Contents

1. [Introduction](#introduction)
2. [CMS Usage Guide](#cms-usage-guide)
   - [Accessing the CMS](#accessing-the-cms)
   - [Updating the Hero Section](#updating-the-hero-section)
   - [Managing Features](#managing-features)
   - [Updating FAQ](#updating-faq)
   - [Adding Announcements](#adding-announcements)
3. [Content Approval Workflow](#content-approval-workflow)
4. [Newsletter Management](#newsletter-management)
   - [Creating Campaigns](#creating-campaigns)
   - [Managing Subscribers](#managing-subscribers)
   - [Viewing Analytics](#viewing-analytics)
5. [Contact Form Management](#contact-form-management)
   - [Viewing Submissions](#viewing-submissions)
   - [Responding to Inquiries](#responding-to-inquiries)
   - [Spam Handling](#spam-handling)
6. [Waitlist Management](#waitlist-management)
   - [Viewing Signups](#viewing-signups)
   - [Sending Invitations](#sending-invitations)
   - [Exporting Data](#exporting-data)
7. [Media Asset Management](#media-asset-management)
   - [Image Optimization](#image-optimization)
   - [Upload Guidelines](#upload-guidelines)
   - [Asset Organization](#asset-organization)
8. [SEO Content Guidelines](#seo-content-guidelines)
9. [Brand Voice & Tone Guide](#brand-voice--tone-guide)
10. [Content Calendar Template](#content-calendar-template)
11. [Troubleshooting](#troubleshooting)
12. [Video Tutorials & Additional Resources](#video-tutorials--additional-resources)

---

## Introduction

This guide provides step-by-step instructions for non-technical team members to manage content on the PredictIQ platform. The content management system (CMS) is designed to be user-friendly and doesn't require coding knowledge.

### System Overview

The PredictIQ content management system includes:

- **CMS Dashboard** - For managing landing page content (hero, features, FAQ, announcements)
- **Newsletter System** - For creating and sending email campaigns
- **Contact Form Handler** - For managing user inquiries
- **Waitlist Manager** - For handling early access signups

---

## CMS Usage Guide

### Accessing the CMS

**URL:** `https://predictiq.com/admin/content`

**Login Credentials:**
- Email: Your company email
- Password: Set by administrator (reset via "Forgot Password" if needed)

> **Screenshot Location:** `/docs/screenshots/cms-login.png`

---

### Updating the Hero Section

The hero section is the first thing visitors see. It includes the headline, subheadline, CTA buttons, and background image.

#### Step-by-Step Instructions:

1. **Log in** to the CMS dashboard
2. **Navigate** to `Content` → `Landing Page` → `Hero Section`
3. **Edit the following fields:**
   - **Headline** (max 60 characters recommended)
   - **Subheadline** (max 150 characters recommended)
   - **Primary CTA Text** (e.g., "Get Started")
   - **Primary CTA Link** (e.g., `/signup`)
   - **Secondary CTA Text** (e.g., "Learn More")
   - **Secondary CTA Link** (e.g., `/about`)
4. **Upload Background Image:**
   - Click "Upload Image"
   - Select file (recommended: 1920x1080px, PNG or WebP)
   - Click "Insert"
5. **Preview Changes:** Click "Preview" to see how it looks
6. **Save Draft:** Click "Save Draft" to store changes without publishing
7. **Publish:** Click "Publish" to make changes live

> **Screenshot Guide:** `/docs/screenshots/cms-hero-edit.png`

#### Best Practices:

- Keep headlines action-oriented and benefit-driven
- Use contrasting colors for CTAs
- Ensure text is readable over the background image

---

### Managing Features

The features section showcases what PredictIQ offers.

#### Step-by-Step Instructions:

1. **Navigate** to `Content` → `Landing Page` → `Features`
2. **To Add a New Feature:**
   - Click `+ Add Feature`
   - Fill in:
     - **Icon** - Choose from icon library or upload custom
     - **Title** (max 40 characters)
     - **Description** (max 120 characters)
     - **Display Order** - Number to determine position
   - Click `Save`
3. **To Edit a Feature:**
   - Click the feature card
   - Make changes
   - Click `Update`
4. **To Delete a Feature:**
   - Click the feature card
   - Click `Delete` (confirm in popup)
5. **To Reorder Features:**
   - Drag and drop cards to new positions
   - Click `Save Order`

> **Screenshot Guide:** `/docs/screenshots/cms-features-edit.png`

#### Recommended Feature Count:

- Minimum: 3 features
- Maximum: 6 features
- Optimal: 4 features (2x2 grid)

---

### Updating FAQ

The FAQ section addresses common user questions.

#### Step-by-Step Instructions:

1. **Navigate** to `Content` → `Landing Page` → `FAQ`
2. **To Add a New FAQ:**
   - Click `+ Add Question`
   - Fill in:
     - **Question** - The user's question
     - **Answer** - Your response (supports rich text)
     - **Category** - Group related questions (optional)
     - **Order** - Display position
   - Click `Save`
3. **To Edit an FAQ:**
   - Click the question to expand
   - Make changes
   - Click `Update`
4. **To Delete an FAQ:**
   - Click the question to expand
   - Click `Delete`
5. **To Reorder:**
   - Use the drag handles to reorder
   - Click `Save Order`

> **Screenshot Guide:** `/docs/screenshots/cms-faq-edit.png`

---

### Adding Announcements

Announcements are used for temporary messages like promotions, maintenance, or events.

#### Step-by-Step Instructions:

1. **Navigate** to `Content` → `Announcements`
2. **Click `+ New Announcement`**
3. **Fill in fields:**
   - **Title** - Internal name (not displayed)
   - **Message** - The announcement text
   - **Type:**
     - `Info` - Blue, general information
     - `Success` - Green, positive news
     - `Warning` - Yellow, caution needed
     - `Error` - Red, critical issues
   - **Display Location:**
     - `Banner` - Top of all pages
     - `Homepage` - Only on landing page
     - `Modal` - Popup on first visit
   - **Start Date** - When to begin showing
   - **End Date** - When to stop showing
   - **Dismissable` - Allow users to close
4. **Click `Publish`**

> **Screenshot Guide:** `/docs/screenshots/cms-announcement-create.png`

---

## Content Approval Workflow

All content changes follow an approval workflow to ensure quality and consistency.

### Workflow Stages:

```
[Draft] → [Review] → [Approved] → [Published]
                ↓
           [Changes Requested]
```

### Step-by-Step Process:

1. **Create/Edit Content**
   - Make changes in CMS
   - Save as "Draft"

2. **Submit for Review**
   - Click "Submit for Review"
   - Add optional notes for reviewer
   - Select reviewer from dropdown

3. **Review Process**
   - Reviewer receives notification
   - Reviewer examines content
   - Reviewer either:
     - Approves → Content moves to "Approved"
     - Requests Changes → Content returns to "Draft" with feedback

4. **Publish**
   - Once approved, click "Publish"
   - Content goes live immediately
   - Or schedule for future date

### Roles & Permissions:

| Role | Create | Edit Own | Review | Publish |
|------|--------|----------|--------|---------|
| Content Writer | ✅ | ✅ | ❌ | ❌ |
| Editor | ✅ | ✅ | ✅ | ❌ |
| Admin | ✅ | ✅ | ✅ | ✅ |

---

## Newsletter Management

### Creating Campaigns

#### Step-by-Step Instructions:

1. **Navigate** to `Marketing` → `Newsletters` → `Create Campaign`
2. **Campaign Details:**
   - **Campaign Name** - Internal name
   - **Subject Line** - Email subject (max 60 chars)
   - **Preview Text** - Shows in inbox preview
   - **From Name** - Sender display name
   - **Reply-To** - Response email address

3. **Select Recipients:**
   - Choose list: All Subscribers, Active Only, Custom Segment
   - View recipient count

4. **Design Email:**
   - **Template:** Choose from templates or start blank
   - **Add Content:**
     - Drag and drop content blocks
     - Edit text inline
     - Insert images
     - Add buttons
   - **Personalization:**
     - `{{name}}` - Subscriber's name
     - `{{email}}` - Subscriber's email

5. **Schedule or Send:**
   - **Send Now** - Immediate delivery
   - **Schedule** - Set date and time
   - **A/B Test** - Test different subject lines

6. **Review & Send:**
   - Click "Send Test" to receive a sample
   - Review checklist
   - Click "Send" or "Schedule"

> **Screenshot Guide:** `/docs/screenshots/newsletter-create-campaign.png`

---

### Managing Subscribers

#### Viewing Subscribers:

1. **Navigate** to `Marketing` → `Subscribers`
2. **Filter by:**
   - Status: All, Active, Unconfirmed, Unsubscribed
   - Date range
   - Source
3. **Search** by email address

#### Adding Subscribers Manually:

1. Click `+ Add Subscriber`
2. Enter email address
3. Select tags (optional)
4. Click `Add`

#### Bulk Import:

1. Click `Import`
2. Upload CSV file (format: email, name, source)
3. Map columns
4. Review and confirm

#### Unsubscribing:

1. Find subscriber
2. Click their row
3. Click `Unsubscribe`
4. Or use bulk unsubscribe for multiple

---

### Viewing Analytics

#### Dashboard Metrics:

1. **Navigate** to `Marketing` → `Analytics`

2. **Key Metrics:**
   - **Sent** - Total emails sent
   - **Delivered** - Emails successfully delivered
   - **Opened** - Emails opened (with %)
   - **Clicked** - Links clicked (with %)
   - **Bounced** - Failed deliveries (hard/soft)
   - **Unsubscribed** - Unsubscribes

3. **Campaign Performance:**
   - View individual campaign stats
   - Compare performance over time

4. **Export Reports:**
   - Click `Export` for CSV download
   - Select date range
   - Choose metrics to include

> **Video Tutorial:** `/docs/videos/newsletter-analytics.mp4`

---

## Contact Form Management

### Viewing Submissions

#### Step-by-Step Instructions:

1. **Navigate** to `Communications` → `Contact Form` → `Submissions`
2. **View List:**
   - Shows newest first
   - Displays: Name, Email, Subject, Date, Status
3. **Filter by:**
   - Status: New, In Progress, Resolved
   - Date range
   - Subject
4. **View Details:**
   - Click a submission to see full message
   - Includes: Name, Email, Subject, Message, Metadata

> **Screenshot Guide:** `/docs/screenshots/contact-submissions.png`

---

### Responding to Inquiries

#### Step-by-Step Instructions:

1. **Open the submission** from the list
2. **Click `Reply`**
3. **Compose response:**
   - Use templates for common responses
   - Personalize as needed
4. **Click `Send Response`**
5. **Update Status:**
   - Set to "In Progress" while handling
   - Set to "Resolved" when done

#### Response Templates:

Available in `Communications` → `Templates`:

- **Thank You** - Auto-response to confirm receipt
- **General Inquiry** - Standard response
- **Support Request** - Technical help response
- **Partnership** - Business inquiry response
- **Press** - Media inquiry response

---

### Spam Handling

#### Automatic Spam Filter:

The system automatically flags suspicious submissions:

- Contains known spam keywords
- Multiple submissions from same IP
- Suspicious URLs in message

#### Managing Spam:

1. **Navigate** to `Communications` → `Contact Form` → `Spam`
2. **Review flagged items:**
   - Click "Not Spam" to move to main inbox
   - Click "Delete" to remove permanently
3. **Whitelist Senders:**
   - Open submission
   - Click "Add to Whitelist"
   - Future submissions from this email bypass spam filter

#### Spam Prevention Tips:

- Don't display email addresses publicly
- Use form CAPTCHA
- Monitor for suspicious patterns

---

## Waitlist Management

### Viewing Signups

#### Step-by-Step Instructions:

1. **Navigate** to `Marketing` → `Waitlist`
2. **Dashboard shows:**
   - Total signups
   - Pending invitations
   - Conversion rate
3. **List View:**
   - Email, Join Date, Status, Priority Score
4. **Filter by:**
   - Status: Pending, Invited, Converted
   - Join date range
   - Source

> **Screenshot Guide:** `/docs/screenshots/waitlist-dashboard.png`

---

### Sending Invitations

#### Step-by-Step Instructions:

1. **Select recipients:**
   - Check individual boxes
   - Or click "Select All" for bulk
2. **Click `Send Invitations`**
3. **Configure invitation:**
   - Choose email template
   - Add personalized message (optional)
   - Set expiration (default: 7 days)
4. **Review and Send:**
   - Preview emails
   - Click "Send Invitations"

#### Manual Invitation:

1. Find the signup in the list
2. Click their row
3. Click `Send Invitation`
4. Confirm in popup

---

### Exporting Data

#### Export Options:

1. **Navigate** to `Marketing` → `Waitlist` → `Export`
2. **Choose format:**
   - CSV (for Excel/Sheets)
   - JSON (for developers)
   - PDF (for reports)
3. **Select fields:**
   - Email
   - Join date
   - Status
   - Source
   - Priority score
4. **Set date range (optional)**
5. **Click `Export`**

#### Scheduled Exports:

1. **Click `Schedule Export`**
2. **Set frequency:** Daily, Weekly, Monthly
3. **Choose recipients:** Your email or distribution list
4. **Click `Schedule`**

---

## Media Asset Management

### Image Optimization

#### Required Specifications:

| Type | Format | Max Size | Dimensions |
|------|--------|----------|------------|
| Hero Background | WebP, PNG | 500KB | 1920x1080px |
| Feature Icons | SVG, PNG | 100KB | 64x64px or 128x128px |
| Blog Images | WebP, JPEG | 800KB | 1200x630px |
| Team Photos | JPEG | 300KB | 400x400px |

#### Optimization Tools:

- **TinyPNG** - https://tinypng.com (recommended)
- **Squoosh** - https://squoosh.app
- **ImageOptim** - macOS app

#### Automatic Optimization:

The CMS automatically:
- Creates responsive versions
- Generates thumbnails
- Converts to WebP when supported

---

### Upload Guidelines

#### Step-by-Step Instructions:

1. **Navigate** to `Media` → `Library`
2. **Click `Upload`**
3. **Drag files** or click to browse
4. **Add metadata:**
   - Title (required)
   - Alt text (required for accessibility)
   - Description (optional)
   - Tags (optional)
5. **Click `Upload`**

#### File Naming:

Use descriptive, lowercase names:
- ✅ `hero-background-platform-launch.webp`
- ❌ `IMG_1234.png`
- ❌ `Hero background.JPG`

#### Folder Structure:

```
/uploads/
├── hero/
│   ├── 2026/
│   │   └── platform-launch.webp
├── features/
│   └── icons/
├── blog/
│   └── 2026/
│       └── article-name.webp
└── team/
    └── headshots/
```

---

### Asset Organization

#### Best Practices:

1. **Use Consistent Naming** - Establish naming conventions
2. **Add Tags** - For easy searching
3. **Use Folders** - Organize by content type and date
4. **Regular Cleanup** - Remove unused assets quarterly

#### Search & Filter:

- Search by filename, title, or tag
- Filter by type, date, or folder
- Sort by name, date, or size

---

## SEO Content Guidelines

### Writing for Search Engines

#### Title Tags:

- **Length:** 50-60 characters
- **Format:** `Primary Keyword | Brand Name`
- **Example:** `Prediction Markets for Crypto | PredictIQ`

#### Meta Descriptions:

- **Length:** 150-160 characters
- **Include:** Keyword, action word, benefit
- **Example:** "Trade prediction markets on crypto, sports, and politics with PredictIQ. Low fees, instant settlements, and decentralized oracles."

#### Header Tags:

- **H1:** One per page, includes main keyword
- **H2:** Section headers, related keywords
- **H3:** Subsections, long-tail keywords

#### Content Best Practices:

1. **Keyword Research:**
   - Use tools like Google Keyword Planner
   - Target 1-2 primary keywords per page
   - Include 3-5 related keywords naturally

2. **Content Structure:**
   - Use short paragraphs (2-3 sentences)
   - Include bullet points for lists
   - Add headings every 300 words

3. **Links:**
   - Internal links to related content
   - External links to authoritative sources
   - Descriptive anchor text

4. **Images:**
   - Always include alt text
   - Compress for fast loading
   - Use descriptive filenames

### Content Types & SEO:

| Content Type | Update Frequency | SEO Focus |
|--------------|------------------|-----------|
| Homepage | Quarterly | Brand + main keywords |
| Features | Monthly | Product keywords |
| FAQ | As needed | Long-tail questions |
| Blog | Weekly | Informational keywords |
| Announcements | As needed | Event/temporal keywords |

---

## Brand Voice & Tone Guide

### Our Brand Personality

PredictIQ is:

- **Confident** but not arrogant
- **Professional** but approachable  
- **Innovative** but reliable
- **Direct** but friendly

### Voice Characteristics:

#### 1. Clarity First
- Use simple, everyday words
- Avoid jargon unless necessary
- Explain technical terms
- **✅** "Your prediction was correct. You won $50."
- **❌** "Your market position was validated. Payout initiated."

#### 2. Active Voice
- Prefer active over passive
- **✅** "We process withdrawals in minutes"
- **❌** "Withdrawals are processed by our system"

#### 3. Benefit-Driven
- Focus on what users gain
- **✅** "Earn rewards for accurate predictions"
- **❌** "Our platform has a reward system"

#### 4. Be Concise
- Get to the point quickly
- Use short sentences
- Break up long text with bullets

### Tone by Context:

| Context | Tone | Example |
|---------|------|---------|
| Marketing | Exciting, benefit-driven | "Unlock your prediction potential" |
| Support | Helpful, patient | "Here's how to resolve that issue" |
| Error Messages | Apologetic, clear | "Something went wrong. We're fixing it." |
| Success Messages | Celebratory | "Congratulations! You won!" |

### Words We Use:

#### Preferred:
- Predict, trade, win, earn, rewards, insights
- Platform, markets, outcomes, users
- Easy, fast, secure, transparent

#### Avoid:
- Gamble, bet, risk (unless regulatory required)
- Crazy, insane, mind-blowing
- Technically complex terms without explanation

### Writing Examples:

#### Email Subject Lines:
- **✅** "Your PredictIQ weekly digest"
- **❌** "Here's what happened on PredictIQ"

#### CTA Buttons:
- **✅** "Start Trading", "Join Waitlist", "Learn More"
- **❌** "Click Here", "Submit", "Go"

#### Social Media:
- **✅** "Which market will you predict first? 🔮"
- **❌** "Use our platform to gamble on events"

---

## Content Calendar Template

### Monthly Planning Template

| Week | Content Type | Topic | Channel | Owner | Status | Publish Date |
|------|--------------|-------|---------|-------|--------|--------------|
| 1 | Blog Post | Market analysis | Website, Social | @name | Draft | Jan 1 |
| 1 | Social Post | Feature highlight | Twitter, LinkedIn | @name | Scheduled | Jan 2 |
| 2 | Newsletter | Monthly roundup | Email | @name | Planned | Jan 8 |
| 2 | FAQ Update | New questions | Website | @name | In Review | Jan 10 |
| 3 | Blog Post | How-to guide | Website, Social | @name | Draft | Jan 15 |
| 3 | Announcement | Platform update | Banner, Email | @name | Approved | Jan 18 |
| 4 | Case Study | User success story | Website | @name | Research | Jan 25 |
| 4 | Social Post | Community spotlight | Twitter | @name | Planned | Jan 30 |

### Content Types Guide:

| Type | Frequency | Effort | Impact |
|------|-----------|--------|--------|
| Blog Posts | Weekly | High | High |
| Social Updates | Daily | Low | Medium |
| Newsletter | Bi-weekly | Medium | High |
| FAQ Updates | As needed | Low | Medium |
| Announcements | As needed | Low | High |
| Video Content | Monthly | Very High | High |

### Workflow Checklist:

- [ ] Topic selected
- [ ] Keyword research done
- [ ] Outline approved
- [ ] First draft complete
- [ ] Internal review done
- [ ] SEO optimization applied
- [ ] Brand voice check
- [ ] Images/media ready
- [ ] Links verified
- [ ] Published
- [ ] Promoted
- [ ] Performance tracked

---

## Troubleshooting

### Common Issues & Solutions

#### CMS Issues

| Problem | Solution |
|---------|----------|
| Can't log in | Use "Forgot Password" or contact admin |
| Changes not showing | Clear browser cache or check published status |
| Image won't upload | Check file size/type, try reformatting |
| Can't save | Check required fields, try different browser |

#### Newsletter Issues

| Problem | Solution |
|---------|----------|
| Emails not sending | Check SMTP settings, verify credits |
| High bounce rate | Clean subscriber list, verify emails |
| Low open rate | Test subject lines, check send time |
| Unsubscribe not working | Verify link in template |

#### Contact Form Issues

| Problem | Solution |
|---------|----------|
| Not receiving submissions | Check email notifications, spam folder |
| Spam getting through | Enable stricter filtering |
| Missing submissions | Check date filters, contact support |

#### Waitlist Issues

| Problem | Solution |
|---------|----------|
| Invitations not sending | Check email quota, verify template |
| Low conversion | Review invitation timing, offer |
| Data export failing | Reduce date range, try different format |

### Getting Help

1. **Internal Wiki:** `/wiki/content-management`
2. **IT Support:** `support@predictiq.com`
3. **Emergency:** `oncall@predictiq.com`

---

## Video Tutorials & Additional Resources

### Video Tutorials

| Topic | Duration | Link |
|-------|----------|------|
| CMS Basics | 5 min | `/docs/videos/cms-basics.mp4` |
| Hero Section Update | 3 min | `/docs/videos/hero-update.mp4` |
| Newsletter Creation | 8 min | `/docs/videos/newsletter-create.mp4` |
| Contact Form Management | 4 min | `/docs/videos/contact-management.mp4` |
| Waitlist Overview | 5 min | `/docs/videos/waitlist-guide.mp4` |
| Media Library | 4 min | `/docs/videos/media-library.mp4` |

### Screenshot Guides

| Topic | Location |
|-------|----------|
| CMS Login | `/docs/screenshots/cms-login.png` |
| Hero Editor | `/docs/screenshots/cms-hero-edit.png` |
| Features Management | `/docs/screenshots/cms-features-edit.png` |
| FAQ Editor | `/docs/screenshots/cms-faq-edit.png` |
| Announcement Create | `/docs/screenshots/cms-announcement-create.png` |
| Newsletter Dashboard | `/docs/screenshots/newsletter-dashboard.png` |
| Subscriber List | `/docs/screenshots/subscribers-list.png` |
| Contact Submissions | `/docs/screenshots/contact-submissions.png` |
| Waitlist Dashboard | `/docs/screenshots/waitlist-dashboard.png` |
| Media Library | `/docs/screenshots/media-library.png` |

### Templates

| Template | Location |
|----------|----------|
| Content Calendar | `/docs/templates/content-calendar.xlsx` |
| Blog Post Outline | `/docs/templates/blog-outline.docx` |
| Newsletter Template | `/docs/templates/newsletter-template.html` |
| Press Release | `/docs/templates/press-release.docx` |
| Social Media Calendar | `/docs/templates/social-calendar.xlsx` |

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | Feb 2026 | Content Team | Initial release |

---

*Last reviewed: February 2026*
