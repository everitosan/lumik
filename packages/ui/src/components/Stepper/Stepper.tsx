import type { CSSProperties, ReactNode } from 'react';

export interface Step {
  /** Unique identifier for the step */
  id: string;
  /** Label displayed for the step */
  label: string;
  /** Optional icon to display instead of number when completed */
  completedIcon?: ReactNode;
}

export interface StepperProps {
  /** Array of steps to display */
  steps: Step[];
  /** Index of the current active step (0-based) */
  currentStep: number;
  /** Callback when a step is clicked (only completed steps are clickable) */
  onStepClick?: (stepIndex: number) => void;
  /** Orientation of the stepper */
  orientation?: 'horizontal' | 'vertical';
  /** Size variant */
  size?: 'sm' | 'md' | 'lg';
}

const containerStyles: Record<string, CSSProperties> = {
  horizontal: {
    display: 'flex',
    alignItems: 'center',
    gap: '0',
    width: '100%',
  },
  vertical: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0',
  },
};

const stepWrapperStyles: Record<string, CSSProperties> = {
  horizontal: {
    display: 'flex',
    alignItems: 'center',
    flex: 1,
  },
  vertical: {
    display: 'flex',
    alignItems: 'flex-start',
  },
};

const sizeConfig = {
  sm: { circle: 24, font: 12, gap: 6 },
  md: { circle: 32, font: 14, gap: 8 },
  lg: { circle: 40, font: 16, gap: 12 },
};

const CheckIcon = ({ size }: { size: number }) => (
  <svg
    width={size * 0.5}
    height={size * 0.5}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="3"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <polyline points="20 6 9 17 4 12" />
  </svg>
);

export function Stepper({
  steps,
  currentStep,
  onStepClick,
  orientation = 'horizontal',
  size = 'md',
}: StepperProps) {
  const config = sizeConfig[size];

  const getStepStatus = (index: number): 'completed' | 'active' | 'pending' => {
    if (index < currentStep) return 'completed';
    if (index === currentStep) return 'active';
    return 'pending';
  };

  const getCircleStyles = (status: 'completed' | 'active' | 'pending'): CSSProperties => {
    const base: CSSProperties = {
      width: config.circle,
      height: config.circle,
      borderRadius: '50%',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: 'var(--lumik-font-primary, Inter)',
      fontSize: config.font,
      fontWeight: 600,
      transition: 'var(--lumik-transition-fast, 150ms ease)',
      flexShrink: 0,
    };

    switch (status) {
      case 'completed':
        return {
          ...base,
          backgroundColor: 'var(--lumik-secondary, #e9c349)',
          color: 'var(--lumik-on-secondary, #3c2f00)',
        };
      case 'active':
        return {
          ...base,
          backgroundColor: 'var(--lumik-primary-container, #558dff)',
          color: 'var(--lumik-on-primary, #002d6e)',
        };
      case 'pending':
        return {
          ...base,
          backgroundColor: 'var(--lumik-surface-container-high, #2a2a2a)',
          color: 'var(--lumik-on-surface-variant, #c2c6d7)',
        };
    }
  };

  const getLabelStyles = (status: 'completed' | 'active' | 'pending'): CSSProperties => {
    const base: CSSProperties = {
      fontFamily: 'var(--lumik-font-primary, Inter)',
      fontSize: config.font,
      fontWeight: 500,
      marginLeft: config.gap,
      whiteSpace: 'nowrap',
      transition: 'var(--lumik-transition-fast, 150ms ease)',
    };

    switch (status) {
      case 'completed':
        return {
          ...base,
          color: 'var(--lumik-secondary, #e9c349)',
        };
      case 'active':
        return {
          ...base,
          color: 'var(--lumik-primary, #b0c6ff)',
        };
      case 'pending':
        return {
          ...base,
          color: 'var(--lumik-on-surface-variant, #c2c6d7)',
        };
    }
  };

  const getConnectorStyles = (status: 'completed' | 'active' | 'pending'): CSSProperties => {
    const isHorizontal = orientation === 'horizontal';

    const base: CSSProperties = isHorizontal
      ? {
          flex: 1,
          height: 2,
          marginLeft: config.gap,
          marginRight: config.gap,
          minWidth: 24,
        }
      : {
          width: 2,
          height: 24,
          marginLeft: config.circle / 2 - 1,
          marginTop: config.gap,
          marginBottom: config.gap,
        };

    return {
      ...base,
      backgroundColor:
        status === 'completed'
          ? 'var(--lumik-secondary, #e9c349)'
          : 'var(--lumik-outline-variant, #424654)',
      transition: 'var(--lumik-transition-fast, 150ms ease)',
    };
  };

  const handleStepClick = (index: number) => {
    if (onStepClick && index < currentStep) {
      onStepClick(index);
    }
  };

  return (
    <div style={containerStyles[orientation]}>
      {steps.map((step, index) => {
        const status = getStepStatus(index);
        const isLast = index === steps.length - 1;
        const isClickable = status === 'completed' && onStepClick;

        const stepContentStyles: CSSProperties = {
          display: 'flex',
          alignItems: 'center',
          cursor: isClickable ? 'pointer' : 'default',
        };

        return (
          <div
            key={step.id}
            style={{
              ...stepWrapperStyles[orientation],
              ...(orientation === 'vertical' && !isLast && { flexDirection: 'column' }),
            }}
          >
            <div
              style={stepContentStyles}
              onClick={() => handleStepClick(index)}
              role={isClickable ? 'button' : undefined}
              tabIndex={isClickable ? 0 : undefined}
              onKeyDown={(e) => {
                if (isClickable && (e.key === 'Enter' || e.key === ' ')) {
                  handleStepClick(index);
                }
              }}
            >
              <div style={getCircleStyles(status)}>
                {status === 'completed' ? (
                  step.completedIcon || <CheckIcon size={config.circle} />
                ) : (
                  index + 1
                )}
              </div>
              <span style={getLabelStyles(status)}>{step.label}</span>
            </div>

            {!isLast && orientation === 'horizontal' && (
              <div style={getConnectorStyles(status)} />
            )}

            {!isLast && orientation === 'vertical' && (
              <div style={getConnectorStyles(status)} />
            )}
          </div>
        );
      })}
    </div>
  );
}
