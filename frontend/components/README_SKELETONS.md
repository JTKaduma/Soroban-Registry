# Loading Skeleton Components

This directory contains reusable skeleton loading components that provide visual feedback during data fetching.

## Components

### LoadingSkeleton
Base skeleton component with configurable dimensions and variants.

**Props:**
- `width`: string (default: '100%') - Width of the skeleton
- `height`: string (default: '1rem') - Height of the skeleton
- `className`: string - Additional CSS classes
- `variant`: 'rectangular' | 'circular' | 'text' - Shape variant

**Usage:**
```tsx
<LoadingSkeleton width="60%" height="1.5rem" />
<LoadingSkeleton variant="circular" width="3rem" height="3rem" />
```

### ContractCardSkeleton
Skeleton loader matching the ContractCard layout.

**Usage:**
```tsx
{isLoading && <ContractCardSkeleton />}
```

### ExampleCardSkeleton
Skeleton loader matching the ExampleCard layout.

**Usage:**
```tsx
{isLoading && <ExampleCardSkeleton />}
```

### TemplateCardSkeleton
Skeleton loader matching the TemplateCard layout.

**Usage:**
```tsx
{isLoading && <TemplateCardSkeleton />}
```

## Features

- ✅ Smooth pulse animation (2s duration)
- ✅ Theme-aware (light/dark mode support)
- ✅ Accessible (ARIA labels and screen reader support)
- ✅ Responsive design
- ✅ Consistent with design system colors

## Implementation

The skeletons are implemented using:
- Tailwind CSS for styling
- CSS animations for the pulse effect
- Semantic HTML with proper ARIA attributes
- Theme variables for consistent colors

## Animation

The pulse animation is defined in `globals.css`:
```css
@keyframes skeleton-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

## Accessibility

All skeleton components include:
- `role="status"` - Indicates loading state
- `aria-label="Loading content"` - Descriptive label
- `aria-live="polite"` - Announces changes to screen readers
- `<span className="sr-only">Loading...</span>` - Screen reader text
