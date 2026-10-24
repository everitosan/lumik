import { useState, useMemo, useCallback, useEffect, useRef, type ChangeEvent, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Stepper,
  Button,
  Checkbox,
  Icon,
  Input,
  DropZone,
  DropZoneSummary,
  FileList,
  isAllowedRawFile,
  type Step,
  type Drive,
} from '@lumik/ui';
import { open } from '@tauri-apps/plugin-dialog';
import { stat } from '@tauri-apps/plugin-fs';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useConnectedDevices, useProjectsDashboard, useActivePhotographer, useAppSettings, useImport, usePlatform, type ImportRetryStep } from '../lib/hooks';
import { analyzeImportSelection, getPhotographerMetadata, getProjectDeliveryMode } from '../lib/api';
import type { DeliveryMode, FailedFile, ImportBreakdown, ImportLogEntry, ImportProgress, ImportResult, ImportSessionSettings, SkippedFile } from '../lib/types';
import { DeliveryModeSelect } from '../components/DeliveryModeSelect';
import { useDeliveryChange } from '../components/DeliveryChangeDialog';

const pathBasename = (p: string) => p.replace(/\\/g, '/').split('/').pop() || p;

// Convert an Android content URI to a real file path so the Rust backend can open it.
// Works for com.android.externalstorage.documents URIs (SD card / USB-OTG):
//   content://.../document/primary%3ADCIM%2FIMG.CR2  →  /storage/emulated/0/DCIM/IMG.CR2
//   content://.../document/1234-ABCD%3ADCIM%2FIMG.CR2  →  /storage/1234-ABCD/DCIM/IMG.CR2
// Returns null for unrecognized URI schemes (caller should keep original path).
function resolveContentUri(uri: string): string | null {
  if (!uri.startsWith('content://')) return null;
  try {
    const match = uri.match(/\/document\/(.+)$/);
    if (!match) return null;
    const docId = decodeURIComponent(match[1]);
    const colonIdx = docId.indexOf(':');
    if (colonIdx === -1) return null;
    const volume = docId.substring(0, colonIdx);
    const rel = docId.substring(colonIdx + 1);
    const base = volume === 'primary' ? '/storage/emulated/0' : `/storage/${volume}`;
    return `${base}/${rel}`;
  } catch {
    return null;
  }
}

// Misma regla que `sanitize_name` en Rust (import/pipeline.rs): letras y dígitos
// Unicode, `-` y `_` se conservan; todo lo demás pasa a `_`.
const sanitizeProjectName = (name: string) => name.replace(/[^\p{L}\p{N}_-]/gu, '_');

const VIDEO_EXTENSIONS = ['.mp4', '.mov', '.avi', '.mts', '.m2ts', '.mkv', '.mxf'];
const isVideoFile = (filename: string) =>
  VIDEO_EXTENSIONS.some((ext) => filename.toLowerCase().endsWith(ext));

interface SourceFile {
  name: string;
  sizeBytes: number;
  path: string;
}

interface ImportState {
  sourceFiles: SourceFile[];
}

// Styles
const pageStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
  backgroundColor: 'var(--lumik-background, #131313)',
  overflow: 'hidden',
};

const headerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '24px',
  padding: '32px 40px 24px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
};

const titleStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '24px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  margin: 0,
};

const stepperContainerStyles: CSSProperties = {
  maxWidth: '480px',
};

const contentStyles: CSSProperties = {
  flex: 1,
  overflow: 'auto',
  padding: '32px 40px',
};

const footerStyles: CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  padding: '16px 40px',
  borderTop: '1px solid var(--lumik-outline-variant, #424654)',
  backgroundColor: 'var(--lumik-surface-container-low, #1c1b1b)',
};

const footerLeftStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
};

const footerRightStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
};

// Step content styles
const stepContentStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '32px',
  // maxWidth: '600px',
};

const sectionStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '16px',
};

const sectionTitleStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
};

const validationCardStyles = (isValid: boolean): CSSProperties => ({
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '12px 16px',
  backgroundColor: isValid
    ? 'rgba(233, 195, 73, 0.08)'
    : 'rgba(255, 180, 171, 0.08)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: `1px solid ${
    isValid
      ? 'rgba(233, 195, 73, 0.3)'
      : 'rgba(255, 180, 171, 0.3)'
  }`,
});

const validationIconStyles = (isValid: boolean): CSSProperties => ({
  width: '32px',
  height: '32px',
  borderRadius: '50%',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  backgroundColor: isValid
    ? 'rgba(233, 195, 73, 0.15)'
    : 'rgba(255, 180, 171, 0.15)',
  flexShrink: 0,
});

const validationTextStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

// Hint text styles
const hintStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  color: 'var(--lumik-outline, #8c90a0)',
};

// Settings step styles
const settingsHintStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  margin: 0,
};

const renameExampleStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
  marginLeft: '32px',
  marginTop: '-12px',
};

const renameExampleLabelStyles: CSSProperties = {
  fontSize: '12px',
  color: 'var(--lumik-outline, #8c90a0)',
};

const renameExampleCodeStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  color: 'var(--lumik-outline, #8c90a0)',
};

const fieldsGridStyles: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(2, 1fr)',
  gap: '16px',
};

const breakdownStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
};

const breakdownTitleStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

// Confirm step styles
const summaryCardStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '16px',
  padding: '20px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
};

const summaryRowStyles: CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
};

const summaryLabelStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  color: 'var(--lumik-outline, #8c90a0)',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
};

const summaryValueStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  fontWeight: 500,
};

// Progress styles
const progressContainerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '16px',
  padding: '20px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
};

const progressBarContainerStyles: CSSProperties = {
  width: '100%',
  height: '8px',
  backgroundColor: 'var(--lumik-surface-container-high, #2b2a2a)',
  borderRadius: '4px',
  overflow: 'hidden',
};

const progressBarStyles: CSSProperties = {
  height: '100%',
  backgroundColor: 'var(--lumik-secondary, #e9c349)',
  borderRadius: '4px',
  transition: 'width 0.3s ease',
};

const progressTextStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const progressCountStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  color: 'var(--lumik-outline, #8c90a0)',
};

// Import log styles
const importLogContainerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
};

const importLogScrollStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '11px',
  backgroundColor: 'var(--lumik-surface-container-high, #2b2a2a)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  padding: '12px',
  maxHeight: '180px',
  overflowY: 'auto',
  display: 'flex',
  flexDirection: 'column',
  gap: '3px',
};

const resultSuccessStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '16px',
  backgroundColor: 'rgba(233, 195, 73, 0.08)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: '1px solid rgba(233, 195, 73, 0.3)',
};

const failedHeaderStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '12px',
  flexWrap: 'wrap',
};

const resultErrorStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '16px',
  backgroundColor: 'rgba(255, 180, 171, 0.08)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: '1px solid rgba(255, 180, 171, 0.3)',
};

const resultWarningStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'flex-start',
  gap: '12px',
  padding: '16px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
};

const fileListStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  maxHeight: '240px',
  overflowY: 'auto',
};

const fileRowStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '2px',
};

const fileDetailStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '11px',
  color: 'var(--lumik-outline, #8c90a0)',
  wordBreak: 'break-all',
};

const progressRowStyles: CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'baseline',
  gap: '12px',
};

/** Fallidos que se muestran antes de "Ver todos". */
const FAILED_PREVIEW = 3;

/** Fases en las que la barra avanza por bytes copiados desde el origen. */
const BYTE_PHASES = new Set(['videos', 'copy', 'copy_metadata']);

function generateSessionId(): string {
  return `import-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
}

// Helper functions
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function formatElapsedTime(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;

  if (minutes > 0) {
    return `${minutes}m ${remainingSeconds}s`;
  }
  return `${seconds}s`;
}

interface ImportPageProps {
  /** Proyecto destino: el import siempre se abre desde un proyecto */
  initialProjectId: string;
  initialProjectName: string;
  /** Disco donde vive el proyecto */
  initialDeviceId: string;
  /** Vuelve al detalle del proyecto */
  onClose: () => void;
}

export function ImportPage({
  initialProjectId,
  initialProjectName,
  initialDeviceId,
  onClose,
}: ImportPageProps) {
  const { t } = useTranslation();

  const IMPORT_STEPS: Step[] = [
    { id: 'origin', label: t('import.steps.origin') },
    { id: 'settings', label: t('import.steps.settings') },
    { id: 'confirm', label: t('import.steps.confirm') },
  ];

  const [currentStepIndex, setCurrentStepIndex] = useState(0);
  const logEndRef = useRef<HTMLDivElement>(null);
  const [importState, setImportState] = useState<ImportState>({
    sourceFiles: [],
  });

  // Import timing
  const [elapsedTime, setElapsedTime] = useState<number | null>(null);

  // Ajustes de esta importación: parten de los globales y no los modifican.
  const [sessionSettings, setSessionSettings] = useState<ImportSessionSettings | null>(null);
  const globalSettingsRef = useRef<ImportSessionSettings | null>(null);

  const { data: devices, refetch: refetchDevices } = useConnectedDevices();
  const { data: projects } = useProjectsDashboard();
  const { data: photographer } = useActivePhotographer();
  const { data: appSettings } = useAppSettings();
  const { status: importStatus, progress, importLog, result, isImporting, error: importError, startImport, retry } = useImport();
  const [showAllFailed, setShowAllFailed] = useState(false);
  const platform = usePlatform();

  // Desglose por tomas de lo seleccionado (pares RAW+JPG, sueltos, uniones).
  const [breakdown, setBreakdown] = useState<ImportBreakdown | null>(null);
  const [analyzing, setAnalyzing] = useState(false);

  // Entrega al aprobar: es del proyecto, no de esta importación.
  const [delivery, setDelivery] = useState<DeliveryMode | null>(null);
  useEffect(() => {
    getProjectDeliveryMode(initialProjectId)
      .then(setDelivery)
      .catch((err) => console.error('getProjectDeliveryMode failed:', err));
  }, [initialProjectId]);

  // Cambiarla avisa si afecta a fotos ya aprobadas; cancelar deja la anterior.
  const { requestChange, dialog: deliveryDialog } = useDeliveryChange(initialProjectId);
  const handleDeliveryChange = (mode: DeliveryMode) => {
    const previous = delivery;
    setDelivery(mode);
    requestChange(mode)
      .then((saved) => {
        if (!saved) setDelivery(previous);
      })
      .catch((err) => {
        console.error('set delivery mode failed:', err);
        setDelivery(previous);
      });
  };

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [importLog]);

  // Arma los valores iniciales una sola vez, cuando llegan ajustes y fotógrafo.
  useEffect(() => {
    if (!appSettings || !photographer || globalSettingsRef.current) return;
    let cancelled = false;
    getPhotographerMetadata(photographer.id)
      .catch(() => null)
      .then((metadata) => {
        if (cancelled || globalSettingsRef.current) return;
        const initial: ImportSessionSettings = {
          rename_on_import: appSettings.rename_on_import,
          embed_metadata: appSettings.embed_metadata_on_import,
          artist: metadata?.artist ?? '',
          copyright: metadata?.copyright ?? '',
          contact_url: metadata?.contact_url ?? '',
        };
        globalSettingsRef.current = initial;
        setSessionSettings(initial);
      });
    return () => {
      cancelled = true;
    };
  }, [appSettings, photographer]);

  const updateSessionSettings = (patch: Partial<ImportSessionSettings>) => {
    setSessionSettings((s) => (s ? { ...s, ...patch } : s));
  };

  const handleResetSettings = () => {
    setSessionSettings(globalSettingsRef.current);
  };

  // Tauri intercepts OS drag-drop before the WebView sees the files.
  // dataTransfer.files is empty in the HTML5 drop event when dragDropEnabled is true.
  // We use onDragDropEvent to receive the real OS paths instead.
  const currentStepIndexRef = useRef(currentStepIndex);
  useEffect(() => {
    currentStepIndexRef.current = currentStepIndex;
  }, [currentStepIndex]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    getCurrentWindow().onDragDropEvent(async (event) => {
      if (event.payload.type !== 'drop') return;
      if (currentStepIndexRef.current !== 0) return;

      const paths: string[] = event.payload.paths;
      const validPaths = paths.filter((filePath) => {
        const name = pathBasename(filePath);
        return isAllowedRawFile(name);
      });
      if (validPaths.length === 0) return;

      try {
        const newFiles: SourceFile[] = await Promise.all(
          validPaths.map(async (filePath) => {
            const fileInfo = await stat(filePath);
            const name = pathBasename(filePath);
            return { name, sizeBytes: fileInfo.size, path: filePath };
          })
        );

        setImportState((s) => {
          const existingPaths = new Set(s.sourceFiles.map((f) => f.path));
          const uniqueNew = newFiles.filter((f) => !existingPaths.has(f.path));
          return { ...s, sourceFiles: [...s.sourceFiles, ...uniqueNew] };
        });
      } catch (err) {
        console.error('Error processing dropped files:', err);
      }
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, []);

  // Convert devices to Drive format (disk info for the project's device)
  const drives: Drive[] = useMemo(() => {
    if (!devices) return [];
    return devices.map((device) => ({
      id: device.uuid,
      name: device.name,
      uuid: device.uuid,
      totalBytes: device.total_bytes ?? 0,
      usedBytes: (device.total_bytes ?? 0) - (device.available_bytes ?? 0),
      connected: true,
      mountPath: device.mount_point,
    }));
  }, [devices]);

  // Calculate total import size
  const totalImportSize = useMemo(() => {
    return importState.sourceFiles.reduce((sum, f) => sum + f.sizeBytes, 0);
  }, [importState.sourceFiles]);

  const photoCount = useMemo(
    () => importState.sourceFiles.filter((f) => !isVideoFile(f.name)).length,
    [importState.sourceFiles]
  );

  const videoCount = useMemo(
    () => importState.sourceFiles.filter((f) => isVideoFile(f.name)).length,
    [importState.sourceFiles]
  );

  // Recalcula el desglose al cambiar la selección (con una pausa corta para no
  // leer las fechas de captura en cada archivo que se arrastra).
  useEffect(() => {
    const paths = importState.sourceFiles.map((f) => f.path);
    if (paths.length === 0) {
      setBreakdown(null);
      return;
    }
    let cancelled = false;
    setAnalyzing(true);
    const timer = setTimeout(() => {
      analyzeImportSelection(initialProjectId, paths)
        .then((b) => !cancelled && setBreakdown(b))
        .catch((err) => {
          console.error('analyzeImportSelection failed:', err);
          if (!cancelled) setBreakdown(null);
        })
        .finally(() => !cancelled && setAnalyzing(false));
    }, 300);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [importState.sourceFiles, initialProjectId]);

  /** Fotos nuevas: por toma si ya hay desglose, si no por archivo. */
  const shotCount = breakdown?.photos ?? photoCount;

  // Get selected drive
  const selectedDrive = useMemo(() => {
    return drives.find((d) => d.id === initialDeviceId);
  }, [drives, initialDeviceId]);

  // Get selected project
  const selectedProject = useMemo(() => {
    if (!projects) return null;
    return projects.find((p) => p.id === initialProjectId);
  }, [projects, initialProjectId]);

  // Validation
  const hasEnoughSpace = useMemo(() => {
    if (!selectedDrive) return false;
    const freeBytes = selectedDrive.totalBytes - selectedDrive.usedBytes;
    return freeBytes >= totalImportSize;
  }, [selectedDrive, totalImportSize]);

  const canProceedFromOrigin = importState.sourceFiles.length > 0;

  // Reintento: los `register_failed` ya están en `_media/` y solo se registran;
  // el resto se vuelve a importar desde el origen, y eso necesita espacio.
  const retryPlan = useMemo(() => {
    const failed = result?.failed_files ?? [];
    const register = failed.filter((f) => f.code === 'register_failed');
    const copy = failed.filter((f) => f.code !== 'register_failed');
    const sizes = new Map(importState.sourceFiles.map((f) => [f.path, f.sizeBytes]));
    const copyBytes = copy.reduce((sum, f) => sum + (sizes.get(f.path) ?? 0), 0);
    return { register, copy, copyBytes };
  }, [result, importState.sourceFiles]);

  const freeBytes = selectedDrive ? selectedDrive.totalBytes - selectedDrive.usedBytes : 0;
  const retryHasSpace = retryPlan.copy.length === 0 || (!!selectedDrive && freeBytes >= retryPlan.copyBytes);

  // Al terminar, el espacio libre cambió: se vuelve a leer para validar el reintento.
  useEffect(() => {
    if (importStatus === 'done') refetchDevices();
  }, [importStatus]);

  // Handlers
  const handleStepClick = (stepIndex: number) => {
    // Con un import en curso o terminado, el wizard no vuelve atrás.
    if (importStatus !== 'idle') return;
    if (stepIndex < currentStepIndex) {
      setCurrentStepIndex(stepIndex);
    }
  };

  const handleNext = () => {
    if (currentStepIndex < IMPORT_STEPS.length - 1) {
      setCurrentStepIndex(currentStepIndex + 1);
    }
  };

  const handleBack = () => {
    if (currentStepIndex > 0) {
      setCurrentStepIndex(currentStepIndex - 1);
    }
  };

  // Tauri dialog for selecting RAW files
  // Note: GTK file dialogs on Linux have issues with extension filters,
  // so we validate files after selection using isAllowedRawFile
  const handleBrowse = useCallback(async () => {
    try {
      const selected = await open({
        multiple: true,
        // No filters - GTK on Linux has issues with extension filtering.
        // On Android, omitting filters forces DocumentsUI (full file browser)
        // instead of the Photo Picker, which doesn't show RAW files.
      });

      if (!selected || selected.length === 0) {
        return;
      }

      // On mobile, the file picker returns content URIs. Resolve them to real paths
      // so the Rust backend can open them as regular files.
      const isMobile = platform === 'android' || platform === 'ios';
      const resolvedPaths = isMobile
        ? selected.map((p: string) => resolveContentUri(p) ?? p)
        : selected;

      // Filter only valid RAW files and get file info
      const validPaths = resolvedPaths.filter((filePath: string) => {
        const name = pathBasename(filePath);
        return isAllowedRawFile(name);
      });

      if (validPaths.length === 0) {
        return;
      }

      const newFiles: SourceFile[] = (await Promise.all(
        validPaths.map(async (filePath: string) => {
          try {
            const fileInfo = await stat(filePath);
            const name = pathBasename(filePath);
            return { name, sizeBytes: fileInfo.size, path: filePath };
          } catch (e) {
            console.error(`stat FAILED: ${filePath}`, e);
            return null;
          }
        })
      )).filter((f): f is SourceFile => f !== null);

      setImportState((s) => {
        // Filter out files that are already in the list (by path)
        const existingPaths = new Set(s.sourceFiles.map((f) => f.path));
        const uniqueNewFiles = newFiles.filter((f) => !existingPaths.has(f.path));

        return {
          ...s,
          sourceFiles: [...s.sourceFiles, ...uniqueNewFiles],
        };
      });
    } catch (error) {
      console.error('Error selecting files:', error);
    }
  }, [platform]);

  const handleRemoveFile = (index: number) => {
    setImportState((s) => ({
      ...s,
      sourceFiles: s.sourceFiles.filter((_, i) => i !== index),
    }));
  };

  const handleStartImport = async () => {
    if (!selectedProject || !selectedDrive) {
      return;
    }

    const sessionId = generateSessionId();
    const startTime = Date.now();
    setElapsedTime(null);
    setShowAllFailed(false);

    try {
      await startImport({
        session_id: sessionId,
        source_files: importState.sourceFiles.map((f) => f.path),
        project_id: selectedProject.id,
        device_uuid: selectedDrive.uuid,
        mount_point: selectedDrive.mountPath!,
        project_name: selectedProject.name,
        settings: sessionSettings ?? undefined,
      });
    } catch (e) {
      console.error('startImport error:', e);
    }

    setElapsedTime(Date.now() - startTime);
  };

  const handleRetry = async () => {
    if (!result || !selectedProject || !selectedDrive || !retryHasSpace) return;
    const steps: ImportRetryStep[] = [];
    if (retryPlan.register.length > 0) {
      steps.push({
        kind: 'register',
        request: {
          session_id: generateSessionId(),
          project_id: selectedProject.id,
          project_name: selectedProject.name,
          file_names: retryPlan.register.map((f) => f.name),
        },
        files: retryPlan.register,
      });
    }
    if (retryPlan.copy.length > 0) {
      steps.push({
        kind: 'import',
        request: {
          session_id: generateSessionId(),
          source_files: retryPlan.copy.map((f) => f.path),
          project_id: selectedProject.id,
          device_uuid: selectedDrive.uuid,
          mount_point: selectedDrive.mountPath!,
          project_name: selectedProject.name,
          settings: sessionSettings ?? undefined,
        },
        files: retryPlan.copy,
      });
    }

    const startTime = Date.now();
    setShowAllFailed(false);
    await retry(steps);
    setElapsedTime((prev) => (prev ?? 0) + Date.now() - startTime);
  };

  // Render step content
  const renderStepContent = () => {
    switch (currentStepIndex) {
      case 0: return renderOriginStep();
      case 1: return renderSettingsStep();
      case 2: return renderConfirmStep();
      default: return null;
    }
  };

  /** Fotos por toma: pares RAW+JPG, sueltos y archivos que se unirán. */
  const renderBreakdown = () => {
    if (!breakdown) {
      return analyzing ? <div style={breakdownStyles}><span style={hintStyles}>{t('import.origin.breakdown.analyzing')}</span></div> : null;
    }
    const parts = [
      breakdown.pairs > 0 && t('import.origin.breakdown.pairs', { count: breakdown.pairs }),
      breakdown.raw_only > 0 && t('import.origin.breakdown.rawOnly', { count: breakdown.raw_only }),
      breakdown.jpg_only > 0 && t('import.origin.breakdown.jpgOnly', { count: breakdown.jpg_only }),
      breakdown.other > 0 && t('import.origin.breakdown.other', { count: breakdown.other }),
    ].filter(Boolean);
    return (
      <div style={breakdownStyles}>
        <span style={breakdownTitleStyles}>{t('import.origin.breakdown.photos', { count: breakdown.photos })}</span>
        {parts.length > 0 && <span style={hintStyles}>{parts.join(' · ')}</span>}
        {breakdown.joins > 0 && (
          <span style={hintStyles}>{t('import.origin.breakdown.joins', { count: breakdown.joins })}</span>
        )}
      </div>
    );
  };

  // Step 1: Origin
  const renderOriginStep = () => (
    <div style={stepContentStyles}>
      <div style={sectionStyles}>
        <DropZoneSummary
          fileCount={importState.sourceFiles.length}
          totalBytes={totalImportSize}
        />
        {photoCount > 0 && renderBreakdown()}
        <DropZone
          onBrowse={handleBrowse}
          title={t('import.origin.dragDrop')}
          hint={t('import.origin.supportedFormats')}
        />
      </div>

      <FileList
        title={t('import.origin.selectFiles')}
        files={importState.sourceFiles}
        onRemove={handleRemoveFile}
        searchPlaceholder={t('components.fileList.searchPlaceholder')}
      />
    </div>
  );

  // Step 2: Settings (solo para esta importación)
  const renderSettingsStep = () => {
    if (!sessionSettings) {
      return (
        <div style={stepContentStyles}>
          <span style={hintStyles}>{t('common.loading')}</span>
        </div>
      );
    }
    const signDisabled = !sessionSettings.embed_metadata;
    return (
      <div style={stepContentStyles}>
        <div style={sectionStyles}>
          <span style={sectionTitleStyles}>{t('import.settings.title')}</span>
          <p style={settingsHintStyles}>{t('import.settings.description')}</p>
        </div>

        <div style={summaryCardStyles}>
          <Checkbox
            label={t('import.settings.rename')}
            checked={sessionSettings.rename_on_import}
            onChange={(checked) => updateSessionSettings({ rename_on_import: checked })}
          />
          {sessionSettings.rename_on_import && (
            <div style={renameExampleStyles}>
              <span style={renameExampleLabelStyles}>{t('import.settings.renameExampleLabel')}</span>
              <span style={renameExampleCodeStyles}>
                {t('import.settings.renameExample', { project: sanitizeProjectName(initialProjectName) })}
              </span>
            </div>
          )}

          <Checkbox
            label={t('import.settings.embed')}
            checked={sessionSettings.embed_metadata}
            onChange={(checked) => updateSessionSettings({ embed_metadata: checked })}
          />
          <div style={fieldsGridStyles}>
            <Input
              label={t('settings.photographer.name')}
              placeholder={t('settings.placeholders.photographerName')}
              value={sessionSettings.artist ?? ''}
              onChange={(e: ChangeEvent<HTMLInputElement>) => updateSessionSettings({ artist: e.target.value })}
              fullWidth
              disabled={signDisabled}
            />
            <Input
              label={t('settings.photographer.copyright')}
              placeholder={t('settings.placeholders.copyright')}
              value={sessionSettings.copyright ?? ''}
              onChange={(e: ChangeEvent<HTMLInputElement>) => updateSessionSettings({ copyright: e.target.value })}
              fullWidth
              disabled={signDisabled}
            />
            <div style={{ gridColumn: '1 / -1' }}>
              <Input
                label={t('settings.photographer.credits')}
                placeholder={t('settings.placeholders.website')}
                value={sessionSettings.contact_url ?? ''}
                onChange={(e: ChangeEvent<HTMLInputElement>) => updateSessionSettings({ contact_url: e.target.value })}
                fullWidth
                disabled={signDisabled}
              />
            </div>
          </div>
        </div>

        <div>
          <Button variant="secondary" size="sm" onClick={handleResetSettings}>
            {t('import.settings.reset')}
          </Button>
        </div>

        {delivery && (
          <>
            <div style={sectionStyles}>
              <span style={sectionTitleStyles}>{t('import.settings.projectTitle')}</span>
              <p style={settingsHintStyles}>{t('import.settings.projectDescription')}</p>
            </div>
            <div style={summaryCardStyles}>
              <DeliveryModeSelect value={delivery} onChange={handleDeliveryChange} />
              <p style={settingsHintStyles}>{t('project.delivery.hint')}</p>
            </div>
          </>
        )}
      </div>
    );
  };

  // Step 3: Confirm
  const renderProgress = (p: ImportProgress) => {
    const percent = p.bytes_total > 0 ? Math.min(100, (p.bytes_done / p.bytes_total) * 100) : 0;
    const withBytes = BYTE_PHASES.has(p.phase);
    return (
      <div style={progressContainerStyles}>
        <div style={progressRowStyles}>
          <span style={progressTextStyles}>{t(`import.progress.phases.${p.phase}`)}</span>
          {p.bytes_total > 0 && (
            <span style={progressCountStyles}>
              {formatBytes(Math.min(p.bytes_done, p.bytes_total))} / {formatBytes(p.bytes_total)}
            </span>
          )}
        </div>
        <div style={progressBarContainerStyles}>
          <div style={{ ...progressBarStyles, width: `${percent}%` }} />
        </div>
        {p.file_name && (
          <div style={progressRowStyles}>
            <span style={{ ...progressTextStyles, wordBreak: 'break-all' }}>{p.file_name}</span>
            {withBytes && p.file_bytes_total > 0 && (
              <span style={{ ...progressCountStyles, flexShrink: 0 }}>
                {formatBytes(p.file_bytes)} / {formatBytes(p.file_bytes_total)}
              </span>
            )}
          </div>
        )}
        {p.file_total > 0 && (
          <span style={progressCountStyles}>
            {t('import.progress.fileOf', { index: p.file_index, total: p.file_total })}
          </span>
        )}
      </div>
    );
  };

  const logLine = (entry: ImportLogEntry) => {
    const file = entry.file_name ?? '';
    switch (entry.kind) {
      case 'file_failed':
        return t('import.log.file_failed', { file, reason: t(`import.errors.codes.${entry.code ?? 'unknown'}`) });
      case 'file_skipped':
        return t('import.log.file_skipped', { file, reason: t(`import.result.skipReasons.${entry.detail ?? 'unsupported_format'}`) });
      case 'photos_registered':
      case 'photos_joined':
      case 'thumbnails':
      case 'done':
        return t(`import.log.${entry.kind}`, { count: entry.count ?? 0 });
      default:
        return t(`import.log.${entry.kind}`, { file, detail: entry.detail ?? '' });
    }
  };

  const resultTitle = (r: ImportResult | null) => {
    if (!r) return t('import.result.failedTitle');
    const imported = r.successful + r.videos_copied + (r.joined ?? 0);
    if (r.fatal || (r.failed > 0 && imported === 0)) return t('import.result.failedTitle');
    if (r.failed > 0) return t('import.result.partialTitle');
    return t('import.progress.completed');
  };

  const renderFailedFiles = (files: FailedFile[]) => {
    const visible = showAllFailed ? files : files.slice(0, FAILED_PREVIEW);
    return (
      <div style={resultErrorStyles}>
        <Icon name="x" size="md" color="var(--lumik-error, #ffb4ab)" />
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1, minWidth: 0 }}>
          <div style={failedHeaderStyles}>
            <span style={progressTextStyles}>{t('import.result.failed', { count: files.length })}</span>
            <Button variant="secondary" size="sm" onClick={handleRetry} disabled={isImporting || !retryHasSpace}>
              {t('import.result.retry', { count: files.length })}
            </Button>
          </div>
          {!retryHasSpace && (
            <span style={progressCountStyles}>
              {t('import.result.retryNoSpace', {
                required: formatBytes(retryPlan.copyBytes),
                available: formatBytes(freeBytes),
              })}
            </span>
          )}
          <div style={fileListStyles}>
            {visible.map((f) => (
              <div key={f.path} style={fileRowStyles}>
                <span style={progressCountStyles}>
                  {f.name}: {t(`import.errors.codes.${f.code}`)}
                </span>
                {f.error && <span style={fileDetailStyles}>{f.error}</span>}
              </div>
            ))}
          </div>
          {files.length > FAILED_PREVIEW && (
            <div>
              <Button variant="ghost" size="sm" onClick={() => setShowAllFailed((v) => !v)}>
                {showAllFailed
                  ? t('import.result.showLess')
                  : t('import.result.showAll', { count: files.length })}
              </Button>
            </div>
          )}
        </div>
      </div>
    );
  };

  const renderSkippedFiles = (files: SkippedFile[]) => (
    <div style={resultWarningStyles}>
      <Icon name="info" size="md" color="var(--lumik-outline, #8c90a0)" />
      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1, minWidth: 0 }}>
        <span style={progressTextStyles}>{t('import.result.skipped', { count: files.length })}</span>
        <div style={fileListStyles}>
          {files.map((f) => (
            <span key={f.path} style={progressCountStyles}>
              {f.name}: {t(`import.result.skipReasons.${f.reason}`)}
            </span>
          ))}
        </div>
      </div>
    </div>
  );

  const renderResult = (r: ImportResult) => (
    <>
      {(r.successful > 0 || r.videos_copied > 0 || r.joined > 0) && (
        <div style={resultSuccessStyles}>
          <Icon name="check" size="md" color="var(--lumik-secondary, #e9c349)" />
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            {r.successful > 0 && (
              <span style={progressTextStyles}>
                {r.successful} {t(r.successful === 1 ? 'import.progress.photoImported_one' : 'import.progress.photoImported_other')}
              </span>
            )}
            {r.videos_copied > 0 && (
              <span style={progressTextStyles}>
                {r.videos_copied} {t(r.videos_copied === 1 ? 'import.progress.videoCopied_one' : 'import.progress.videoCopied_other')}
              </span>
            )}
            {r.joined > 0 && (
              <span style={progressTextStyles}>
                {r.joined} {t(r.joined === 1 ? 'import.progress.joined_one' : 'import.progress.joined_other')}
              </span>
            )}
            {elapsedTime && (
              <span style={progressCountStyles}>
                {t('import.progress.totalTime')}: {formatElapsedTime(elapsedTime)}
              </span>
            )}
          </div>
        </div>
      )}
      {r.fatal && (
        <div style={resultErrorStyles}>
          <Icon name="x" size="md" color="var(--lumik-error, #ffb4ab)" />
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', minWidth: 0 }}>
            <span style={progressTextStyles}>
              {t('import.result.fatal', { reason: t(`import.errors.codes.${r.fatal.code}`) })}
            </span>
            <span style={fileDetailStyles}>{r.fatal.detail}</span>
          </div>
        </div>
      )}
      {r.failed_files.length > 0 && renderFailedFiles(r.failed_files)}
      {r.skipped_files.length > 0 && renderSkippedFiles(r.skipped_files)}
    </>
  );

  const renderConfirmStep = () => {
    // Una vez iniciado, el paso muestra el progreso y luego el resultado
    // (también si falló): nunca vuelve a la confirmación.
    if (importStatus !== 'idle') {
      return (
        <div style={stepContentStyles}>
          <div style={sectionStyles}>
            <span style={sectionTitleStyles}>
              {isImporting ? t('import.progress.importing') : resultTitle(result)}
            </span>

            {isImporting && progress && renderProgress(progress)}

            {importStatus === 'done' && result && renderResult(result)}

            {importStatus === 'done' && importError && (
              <div style={resultErrorStyles}>
                <Icon name="x" size="md" color="var(--lumik-error, #ffb4ab)" />
                <span style={progressTextStyles}>
                  {result
                    ? t('import.result.retryError', { error: importError })
                    : t('import.result.couldNotStart', { error: importError })}
                </span>
              </div>
            )}

            {importLog.length > 0 && (
              <div style={importLogContainerStyles}>
                <span style={sectionTitleStyles}>{t('import.log.title')}</span>
                <div style={importLogScrollStyles}>
                  {importLog.map((entry, i) => (
                    <div
                      key={i}
                      style={{
                        color: entry.kind === 'file_failed'
                          ? 'var(--lumik-error, #ffb4ab)'
                          : 'var(--lumik-on-surface-variant, #c2c6d7)',
                        lineHeight: '1.6',
                        whiteSpace: 'pre-wrap',
                        wordBreak: 'break-all',
                      }}
                    >
                      {logLine(entry)}
                    </div>
                  ))}
                  <div ref={logEndRef} />
                </div>
              </div>
            )}
          </div>
        </div>
      );
    }

    // Default confirmation view
    return (
      <div style={stepContentStyles}>
        <div style={sectionStyles}>
          <span style={sectionTitleStyles}>{t('import.summary.title')}</span>
          <div style={summaryCardStyles}>
            {photoCount > 0 && (
              <div style={summaryRowStyles}>
                <span style={summaryLabelStyles}>{t('import.summary.photos')}</span>
                <span style={summaryValueStyles}>
                  {shotCount}
                  {breakdown && breakdown.pairs > 0 && (
                    <span style={hintStyles}>
                      {' '}({t('import.summary.photosDetail', { pairs: breakdown.pairs, raw: breakdown.raw_only, jpg: breakdown.jpg_only })})
                    </span>
                  )}
                </span>
              </div>
            )}
            {videoCount > 0 && (
              <div style={summaryRowStyles}>
                <span style={summaryLabelStyles}>{t('import.summary.videos')}</span>
                <span style={summaryValueStyles}>{videoCount}</span>
              </div>
            )}
            <div style={summaryRowStyles}>
              <span style={summaryLabelStyles}>{t('import.summary.totalSize')}</span>
              <span style={summaryValueStyles}>{formatBytes(totalImportSize)}</span>
            </div>
            <div style={summaryRowStyles}>
              <span style={summaryLabelStyles}>{t('import.summary.project')}</span>
              <span style={summaryValueStyles}>
                {selectedProject?.name ?? initialProjectName ?? t('common.notSelected')}
              </span>
            </div>
            <div style={summaryRowStyles}>
              <span style={summaryLabelStyles}>{t('import.summary.destination')}</span>
              <span style={summaryValueStyles}>
                {selectedDrive?.name ?? t('common.notSelected')}
              </span>
            </div>
            {selectedDrive && (
              <div style={summaryRowStyles}>
                <span style={summaryLabelStyles}>{t('import.summary.availableSpace')}</span>
                <span style={summaryValueStyles}>
                  {formatBytes(selectedDrive.totalBytes - selectedDrive.usedBytes)}
                </span>
              </div>
            )}
            {sessionSettings && (
              <>
                <div style={summaryRowStyles}>
                  <span style={summaryLabelStyles}>{t('import.summary.rename')}</span>
                  <span style={summaryValueStyles}>
                    {sessionSettings.rename_on_import ? t('import.summary.renameYes') : t('import.summary.renameNo')}
                  </span>
                </div>
                {delivery && (
                  <div style={summaryRowStyles}>
                    <span style={summaryLabelStyles}>{t('import.summary.delivery')}</span>
                    <span style={summaryValueStyles}>{t(`project.delivery.modes.${delivery}`)}</span>
                  </div>
                )}
                <div style={summaryRowStyles}>
                  <span style={summaryLabelStyles}>{t('import.summary.signedBy')}</span>
                  <span style={summaryValueStyles}>
                    {sessionSettings.embed_metadata && sessionSettings.artist?.trim()
                      ? sessionSettings.artist.trim()
                      : t('import.summary.notSigned')}
                  </span>
                </div>
              </>
            )}
          </div>
        </div>

        <div style={validationCardStyles(hasEnoughSpace)}>
          <div style={validationIconStyles(hasEnoughSpace)}>
            <Icon
              name={hasEnoughSpace ? 'check' : 'x'}
              size="md"
              color={hasEnoughSpace ? 'var(--lumik-secondary, #e9c349)' : 'var(--lumik-error, #ffb4ab)'}
            />
          </div>
          <span style={validationTextStyles}>
            {hasEnoughSpace
              ? t('import.summary.ready')
              : t('import.summary.notEnoughSpace', {
                  required: formatBytes(totalImportSize),
                  available: formatBytes(selectedDrive ? selectedDrive.totalBytes - selectedDrive.usedBytes : 0),
                })}
          </span>
        </div>
      </div>
    );
  };

  // Determine if we can proceed
  const canProceed =
    currentStepIndex === 0 ? canProceedFromOrigin
      : currentStepIndex === 1 ? sessionSettings !== null
        : hasEnoughSpace;

  return (
    <div style={pageStyles}>
      <header style={headerStyles}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <Button variant="ghost" size="sm" onClick={onClose} leftIcon={<Icon name="chevron-right" size="sm" style={{ transform: 'rotate(180deg)' }} />}>
            {t('import.header.back')}
          </Button>
          <h1 style={titleStyles}>
            {t('import.header.title')}
            <span style={{ fontWeight: 400, color: 'var(--lumik-on-surface-variant, #c2c6d7)', fontSize: '18px', marginLeft: '12px' }}>
              → {initialProjectName}
            </span>
          </h1>
        </div>
        <div style={stepperContainerStyles}>
          <Stepper
            steps={IMPORT_STEPS}
            currentStep={currentStepIndex}
            onStepClick={handleStepClick}
            size="md"
          />
        </div>
      </header>

      <div style={contentStyles}>{renderStepContent()}</div>
      {deliveryDialog}

      <footer style={footerStyles}>
        <div style={footerLeftStyles}>
          {importState.sourceFiles.length > 0 && (
            <span style={hintStyles}>
              {[
                shotCount > 0 && `${shotCount} ${t(shotCount === 1 ? 'import.footer.photo_one' : 'import.footer.photo_other')}`,
                videoCount > 0 && `${videoCount} ${t(videoCount === 1 ? 'import.footer.video_one' : 'import.footer.video_other')}`,
              ].filter(Boolean).join(' • ')} • {formatBytes(totalImportSize)}
            </span>
          )}
        </div>
        <div style={footerRightStyles}>
          {currentStepIndex > 0 && importStatus === 'idle' && (
            <Button variant="secondary" onClick={handleBack}>
              <Icon name="chevron-right" size="sm" style={{ transform: 'rotate(180deg)' }} />
              {t('import.footer.back')}
            </Button>
          )}
          {currentStepIndex < IMPORT_STEPS.length - 1 ? (
            <Button
              variant="primary"
              onClick={handleNext}
              disabled={!canProceed}
            >
              {t('import.footer.next')}
              <Icon name="chevron-right" size="sm" />
            </Button>
          ) : importStatus === 'done' ? (
            <Button variant="primary" onClick={onClose}>
              <Icon name="check" size="sm" />
              {t('import.footer.accept')}
            </Button>
          ) : (
            <Button
              variant="primary"
              onClick={handleStartImport}
              disabled={!canProceed || isImporting}
            >
              <Icon name="import" size="sm" />
              {isImporting ? t('import.progress.importing') : t('import.footer.import')}
            </Button>
          )}
        </div>
      </footer>

    </div>
  );
}
