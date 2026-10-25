import { useTranslation } from 'react-i18next';
import { Select } from '@lumik/ui';
import { DELIVERY_MODES, type DeliveryMode } from '../lib/types';

interface DeliveryModeSelectProps {
  value: DeliveryMode;
  onChange: (mode: DeliveryMode) => void;
  /** Etiqueta propia (p. ej. la de Ajustes); por defecto la de la opción del proyecto. */
  labelKey?: string;
  disabled?: boolean;
}

/**
 * Selector de "Al aprobar, entregar directamente" (nada / pares / todo JPG de
 * cámara). Lo usan Ajustes, Editar proyecto y el paso de ajustes del import.
 */
export function DeliveryModeSelect({ value, onChange, labelKey, disabled }: DeliveryModeSelectProps) {
  const { t } = useTranslation();
  const options = DELIVERY_MODES.map(mode => ({ value: mode, label: t(`project.delivery.modes.${mode}`) }));
  return (
    <Select
      label={t(labelKey ?? 'project.delivery.label')}
      options={options}
      value={value}
      onChange={v => onChange(v as DeliveryMode)}
      disabled={disabled}
      fullWidth
    />
  );
}
