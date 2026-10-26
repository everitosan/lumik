import { useTranslation } from 'react-i18next';
import { Select } from '@lumik/ui';
import type { AnalysisMode } from '../lib/types';

const ANALYSIS_MODES: AnalysisMode[] = ['on_demand', 'on_import'];

interface AnalysisModeSelectProps {
  value: AnalysisMode;
  onChange: (mode: AnalysisMode) => void;
  /** Etiqueta propia (p. ej. la de Ajustes); por defecto la del proyecto. */
  labelKey?: string;
  disabled?: boolean;
}

/**
 * Selector de cuándo se analizan las fotos (al importar o bajo demanda), spec
 * `semantic-search`. Lo usan Ajustes → Búsqueda y el menú del proyecto.
 */
export function AnalysisModeSelect({ value, onChange, labelKey, disabled }: AnalysisModeSelectProps) {
  const { t } = useTranslation();
  const options = ANALYSIS_MODES.map((mode) => ({ value: mode, label: t(`project.analysis.modes.${mode}`) }));
  return (
    <Select
      label={t(labelKey ?? 'project.analysis.label')}
      options={options}
      value={value}
      onChange={(v) => onChange(v as AnalysisMode)}
      disabled={disabled}
      fullWidth
    />
  );
}
