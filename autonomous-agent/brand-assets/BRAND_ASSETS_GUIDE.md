# Luna Brand Assets Guide

## Overview
This guide catalogs all Luna brand assets and provides usage guidelines to maintain visual consistency across all implementations.

## Logo & Primary Branding

### Primary Logo
- **File**: `luna-logo-primary.png`
- **Usage**: Main branding, app headers, marketing materials
- **Format**: PNG with transparency
- **Description**: Stylized crescent moon with tech elements and "LUNA" typography

## Mascot Collection

### Main Character Mascot
- **File**: `luna-mascot-main.png`
- **Usage**: Hero sections, about pages, primary character representation
- **Format**: PNG with transparency
- **Description**: Full-body friendly AI character with celestial moon theme

### Avatar/Profile Version
- **File**: `luna-mascot-avatar.png` 
- **Usage**: User profiles, chat interfaces, small profile images
- **Format**: PNG with transparency
- **Description**: Head and shoulders portrait with crescent moon symbol

### Working/Active Pose
- **File**: `luna-mascot-working.png`
- **Usage**: Dashboard active states, productivity features, automation in progress
- **Format**: PNG with transparency  
- **Description**: Luna multitasking with holographic interfaces and data streams

### Emotion Set
- **File**: `luna-mascot-emotions.png`
- **Usage**: Status indicators, feedback messages, interactive responses
- **Format**: PNG with transparency
- **Description**: 6 distinct emotional expressions in grid layout:
  - Happy/Excited
  - Focused/Working  
  - Surprised/Amazed
  - Confident/Proud
  - Thoughtful/Processing
  - Helpful/Welcoming

### Loading/Processing State
- **File**: `luna-mascot-loading.png`
- **Usage**: Loading screens, processing states, meditation/thinking modes
- **Format**: PNG with transparency
- **Description**: Peaceful meditation pose with rotating data particles

### Success/Celebration
- **File**: `luna-mascot-success.png`
- **Usage**: Task completion, success messages, achievement notifications
- **Format**: PNG with transparency
- **Description**: Victory pose with sparkles and celebratory elements

## Color Palette

### Primary Colors
- **Luna Blue**: `#2563eb` (Deep space blue)
- **Silver**: `#64748b` (Metallic silver)
- **Moonlight**: `#f8fafc` (Soft white)

### Accent Colors
- **Electric Blue**: `#06b6d4` (Highlights and accents)
- **Cosmic Purple**: `#8b5cf6` (Secondary accents)
- **Starlight**: `#fbbf24` (Success and positive states)

## Usage Guidelines

### Logo Usage
- Maintain clear space around logo equal to the height of the crescent moon
- Never stretch or distort the logo
- Use on contrasting backgrounds for optimal readability
- Minimum size: 32px height for digital, 0.5 inch for print

### Mascot Usage
- **Main Character**: Use for primary brand representation and hero sections
- **Avatar**: Perfect for profile pictures and small interface elements  
- **Working**: Show Luna in action for productivity and automation features
- **Emotions**: Use contextually to provide feedback and personality
- **Loading**: Display during processing, loading, or thinking states
- **Success**: Celebrate completed tasks and positive outcomes

### Consistency Rules
- Always use transparent PNG versions for overlays
- Maintain the crescent moon symbol across all variations
- Keep the friendly, approachable personality consistent
- Use the established color palette for brand cohesion

### File Naming Convention
- Format: `luna-[type]-[variant].png`
- Examples: `luna-logo-primary.png`, `luna-mascot-avatar.png`
- Use lowercase with hyphens for web compatibility

## Technical Specifications

### File Formats
- **Primary**: PNG with transparency for all assets
- **Backup**: SVG versions recommended for scalable logo usage
- **Web**: Optimized PNG files under 100KB when possible

### Dimensions
- **Logo**: Square format (1024x1024) optimized
- **Main Mascot**: Square format for versatility
- **Avatar**: Square format for profile usage
- **Working**: Landscape format for dashboard integration
- **Emotions**: Landscape grid format
- **Loading**: Square format for loading animations
- **Success**: Square format for notifications

## Integration Examples

### Dashboard Header
```tsx
<img src="/brand-assets/luna-logo-primary.png" alt="Luna" className="h-8" />
```

### Status Indicator
```tsx
<img src="/brand-assets/luna-mascot-loading.png" alt="Luna is thinking..." className="w-16 h-16" />
```

### Success Message
```tsx
<img src="/brand-assets/luna-mascot-success.png" alt="Task completed!" className="w-12 h-12" />
```

## Brand Personality in Assets

### Visual Characteristics
- **Friendly**: Soft, rounded features and warm expressions
- **Intelligent**: Tech-inspired elements without being intimidating  
- **Capable**: Confident poses showing competence and reliability
- **Approachable**: Always smiling or positive, never stern or cold
- **Professional**: Clean, modern design suitable for business contexts

### Emotional Tone
- Helpful and supportive
- Enthusiastic but not overwhelming
- Competent and reliable
- Warm and personable
- Forward-thinking and innovative

## Future Asset Needs

### Potential Additions
- Animated GIF versions for loading states
- SVG versions of mascots for infinite scalability
- Dark mode variations
- Seasonal/holiday themed variants
- Additional emotional expressions
- Luna interacting with specific tools/interfaces

### Asset Requests
For new asset requests, consider:
1. Brand consistency with existing mascots
2. Appropriate emotional tone
3. Technical requirements (size, format, transparency)
4. Usage context and placement
5. Accessibility considerations

---

**Last Updated**: July 2025  
**Asset Count**: 7 total assets (1 logo + 6 mascot variations)  
**Maintained By**: Luna Brand Team