import { useState, useMemo, useCallback, useEffect, useRef, type CSSProperties } from 'react';
import {
  Stepper,
  Button,
  Icon,
  Select,
  DriveSelector,
  DropZone,
  DropZoneSummary,
  FileList,
  isAllowedRawFile,
  type Step,
  type Drive,
  type SelectOption,
} from '@lumik/ui';
import { open } from '@tauri-apps/plugin-dialog';
import { stat } from '@tauri-apps/plugin-fs';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useConnectedDevices, useProjectsDashboard, useActivePhotographer, useImport, useAppSettings } from '../lib/hooks';
import { createProject } from '../lib/api';
import { CreateProjectModal, type ProjectFormData } from '../components/CreateProjectModal';
import type { ImportPhase, FailedFile } from '../lib/types';

const VIDEO_EXTENSIONS = ['.mp4', '.mov', '.avi', '.mts', '.m2ts', '.mkv', '.mxf'];
const isVideoFile = (filename: string) =>
  VIDEO_EXTENSIONS.some((ext) => filename.toLowerCase().endsWith(ext));

// Import wizard steps
type ImportStep = 'origin' | 'destination' | 'confirm';

interface SourceFile {
  name: string;
  sizeBytes: number;
  path: string;
}

interface ImportState {
  sourceFiles: SourceFile[];
  selectedProjectId: string | null;
  selectedDriveId: string | null;
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

const resultSuccessStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '16px',
  backgroundColor: 'rgba(233, 195, 73, 0.08)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  border: '1px solid rgba(233, 195, 73, 0.3)',
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

// Helper functions
function getPhaseLabel(phase: ImportPhase): string {
  const labels: Record<ImportPhase, string> = {
    reading: 'Leyendo',
    decoding: 'Decodificando RAW',
    converting: 'Convirtiendo a DNG',
    hashing: 'Calculando hash',
    writing: 'Guardando archivo',
    saving: 'Registrando en BD',
    complete: 'Completado',
    failed: 'Error',
  };
  return labels[phase] || phase;
}

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
  /** Pre-select a project — skips the destination step entirely */
  initialProjectId?: string;
  initialProjectName?: string;
  /** Pre-select the device where the project lives */
  initialDeviceId?: string;
  /** Called when the user wants to go back (when launched from ProjectDetail) */
  onClose?: () => void;
}

const ALL_STEPS: Step[] = [
  { id: 'origin', label: 'Origen' },
  { id: 'destination', label: 'Destino' },
  { id: 'confirm', label: 'Confirmar' },
];

const EMBEDDED_STEPS: Step[] = [
  { id: 'origin', label: 'Origen' },
  { id: 'confirm', label: 'Confirmar' },
];

export function ImportPage({
  initialProjectId,
  initialProjectName,
  initialDeviceId,
  onClose,
}: ImportPageProps = {}) {
  const embedded = !!initialProjectId;
  const IMPORT_STEPS = embedded ? EMBEDDED_STEPS : ALL_STEPS;

  const [currentStepIndex, setCurrentStepIndex] = useState(0);
  const [importState, setImportState] = useState<ImportState>({
    sourceFiles: [],
    selectedProjectId: initialProjectId ?? null,
    selectedDriveId: initialDeviceId ?? null,
  });

  // Create project modal state
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // Import timing
  const [elapsedTime, setElapsedTime] = useState<number | null>(null);

  const { data: devices, loading: devicesLoading } = useConnectedDevices();
  const { data: projects, loading: projectsLoading, refetch: refetchProjects } = useProjectsDashboard();
  const { data: photographer } = useActivePhotographer();
  const { data: settings } = useAppSettings();
  const { progress, result, isImporting, error: importError, startImport, reset: resetImport } = useImport();

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
        const name = filePath.split('/').pop() || filePath;
        return isAllowedRawFile(name);
      });
      if (validPaths.length === 0) return;

      try {
        const newFiles: SourceFile[] = await Promise.all(
          validPaths.map(async (filePath) => {
            const fileInfo = await stat(filePath);
            const name = filePath.split('/').pop() || filePath;
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

  // Convert devices to Drive format for DriveSelector
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

  // Convert projects to Select options
  const projectOptions: SelectOption[] = useMemo(() => {
    const options: SelectOption[] = [];
    if (projects) {
      projects.forEach((project) => {
        options.push({
          value: project.id,
          label: project.name,
        });
      });
    }
    options.push({
      value: '__new__',
      label: '+ Crear nuevo proyecto',
      isAction: true,
    });
    return options;
  }, [projects]);

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

  // Get selected drive
  const selectedDrive = useMemo(() => {
    return drives.find((d) => d.id === importState.selectedDriveId);
  }, [drives, importState.selectedDriveId]);

  // Get selected project
  const selectedProject = useMemo(() => {
    if (!projects || !importState.selectedProjectId) return null;
    return projects.find((p) => p.id === importState.selectedProjectId);
  }, [projects, importState.selectedProjectId]);

  // Validation
  const hasEnoughSpace = useMemo(() => {
    if (!selectedDrive) return false;
    const freeBytes = selectedDrive.totalBytes - selectedDrive.usedBytes;
    return freeBytes >= totalImportSize;
  }, [selectedDrive, totalImportSize]);

  const canProceedFromOrigin = importState.sourceFiles.length > 0;
  const canProceedFromDestination =
    importState.selectedProjectId !== null &&
    importState.selectedProjectId !== '__new__' &&
    importState.selectedDriveId !== null &&
    hasEnoughSpace;

  // Handlers
  const handleStepClick = (stepIndex: number) => {
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

  const handleProjectChange = (value: string) => {
    if (value === '__new__') {
      setShowCreateModal(true);
      return;
    }
    setImportState((s) => ({ ...s, selectedProjectId: value }));
  };

  const handleCreateProject = async (data: ProjectFormData) => {
    setIsCreating(true);
    setCreateError(null);

    try {
      const newProject = await createProject({
        name: data.name,
        description: data.description || undefined,
        session_date: data.sessionDate || undefined,
        creator_id: photographer!.id,
        device_uuid: data.deviceUuid,
      });

      // Select the newly created project
      setImportState((s) => ({ ...s, selectedProjectId: newProject.id }));
      setShowCreateModal(false);

      // Refresh projects list
      refetchProjects();
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : 'Error al crear el proyecto');
    } finally {
      setIsCreating(false);
    }
  };

  const handleDriveSelect = (drive: Drive) => {
    setImportState((s) => ({ ...s, selectedDriveId: drive.id }));
  };

  // Tauri dialog for selecting RAW files
  // Note: GTK file dialogs on Linux have issues with extension filters,
  // so we validate files after selection using isAllowedRawFile
  const handleBrowse = useCallback(async () => {
    try {
      const selected = await open({
        multiple: true,
        // No filters - GTK on Linux has issues with extension filtering
      });

      if (!selected || selected.length === 0) return;

      // Filter only valid RAW files and get file info
      const validPaths = selected.filter((filePath: string) => {
        const name = filePath.split('/').pop() || filePath;
        return isAllowedRawFile(name);
      });

      if (validPaths.length === 0) return;

      const newFiles: SourceFile[] = await Promise.all(
        validPaths.map(async (filePath: string) => {
          const fileInfo = await stat(filePath);
          const name = filePath.split('/').pop() || filePath;
          return {
            name,
            sizeBytes: fileInfo.size,
            path: filePath,
          };
        })
      );

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
  }, []);

  const handleRemoveFile = (index: number) => {
    setImportState((s) => ({
      ...s,
      sourceFiles: s.sourceFiles.filter((_, i) => i !== index),
    }));
  };

  const handleStartImport = async () => {
    if (!selectedProject || !selectedDrive) return;

    const sessionId = generateSessionId();
    const startTime = Date.now();
    setElapsedTime(null);

    await startImport({
      session_id: sessionId,
      source_files: importState.sourceFiles.map((f) => f.path),
      project_id: selectedProject.id,
      device_uuid: selectedDrive.uuid,
      mount_point: selectedDrive.mountPath!,
      project_name: selectedProject.name,
    });

    // Calculate elapsed time when import finishes
    setElapsedTime(Date.now() - startTime);
  };

  const handleNewImport = () => {
    resetImport();
    setElapsedTime(null);
    setImportState({
      sourceFiles: [],
      selectedProjectId: initialProjectId ?? null,
      selectedDriveId: initialDeviceId ?? null,
    });
    setCurrentStepIndex(0);
  };

  // Render step content
  const renderStepContent = () => {
    if (embedded) {
      switch (currentStepIndex) {
        case 0: return renderOriginStep();
        case 1: return renderConfirmStep();
        default: return null;
      }
    }
    switch (currentStepIndex) {
      case 0: return renderOriginStep();
      case 1: return renderDestinationStep();
      case 2: return renderConfirmStep();
      default: return null;
    }
  };

  // Step 1: Origin
  const renderOriginStep = () => (
    <div style={stepContentStyles}>
      <div style={sectionStyles}>
        <DropZoneSummary fileCount={importState.sourceFiles.length} totalBytes={totalImportSize} />
        <DropZone
          onBrowse={handleBrowse}
          title="Arrastra tus fotos aquí o haz clic para explorar"
          hint="Compatible con RAW, CR2, CR3, NEF, ARW, RAF, ORF, RW2, DNG, JPEG"
        />
      </div>

      <FileList
        title="Archivos seleccionados"
        files={importState.sourceFiles}
        onRemove={handleRemoveFile}
      />
    </div>
  );

  // Step 2: Destination
  const renderDestinationStep = () => (
    <div style={stepContentStyles}>
      <div style={sectionStyles}>
        <Select
          label="Asignar a proyecto"
          options={projectOptions}
          value={importState.selectedProjectId ?? undefined}
          onChange={handleProjectChange}
          placeholder="Seleccionar proyecto..."
          fullWidth
          disabled={projectsLoading}
        />
      </div>

      <div style={sectionStyles}>
        <DriveSelector
          label="Disco de destino"
          drives={drives}
          selectedId={importState.selectedDriveId ?? undefined}
          onSelect={handleDriveSelect}
          requiredBytes={totalImportSize}
        />
        {devicesLoading && (
          <span style={hintStyles}>Detectando dispositivos...</span>
        )}
      </div>

      {selectedDrive && importState.selectedProjectId && (
        <div style={validationCardStyles(hasEnoughSpace)}>
          <div style={validationIconStyles(hasEnoughSpace)}>
            <Icon
              name={hasEnoughSpace ? 'check' : 'x'}
              size="md"
              color={
                hasEnoughSpace
                  ? 'var(--lumik-secondary, #e9c349)'
                  : 'var(--lumik-error, #ffb4ab)'
              }
            />
          </div>
          <span style={validationTextStyles}>
            {hasEnoughSpace
              ? `Destino validado. Espacio suficiente en ${selectedDrive.name}`
              : `Espacio insuficiente en ${selectedDrive.name}. Necesitas ${formatBytes(totalImportSize)}`}
          </span>
        </div>
      )}
    </div>
  );

  // Step 3: Confirm
  const renderConfirmStep = () => {
    // Show import progress if importing or has result
    if (isImporting || result) {
      return (
        <div style={stepContentStyles}>
          <div style={sectionStyles}>
            <span style={sectionTitleStyles}>
              {isImporting ? 'Importando...' : 'Importación completada'}
            </span>

            {/* Progress UI */}
            {isImporting && progress && (
              <div style={progressContainerStyles}>
                <div style={progressBarContainerStyles}>
                  <div
                    style={{
                      ...progressBarStyles,
                      width: `${((progress.current_index + 1) / progress.total_files) * 100}%`,
                    }}
                  />
                </div>
                <span style={progressTextStyles}>
                  {progress.current_file}
                </span>
                <span style={progressCountStyles}>
                  {progress.current_index + 1} de {progress.total_files} pasos
                </span>
              </div>
            )}

            {/* Result UI */}
            {result && (
              <>
                {result.failed === 0 ? (
                  <div style={resultSuccessStyles}>
                    <Icon name="check" size="md" color="var(--lumik-secondary, #e9c349)" />
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                      {result.successful > 0 && (
                        <span style={progressTextStyles}>
                          {result.successful} {result.successful === 1 ? 'fotografía importada' : 'fotografías importadas'}
                        </span>
                      )}
                      {result.videos_copied > 0 && (
                        <span style={progressTextStyles}>
                          {result.videos_copied} {result.videos_copied === 1 ? 'video copiado' : 'videos copiados'}
                        </span>
                      )}
                      {elapsedTime && (
                        <span style={progressCountStyles}>
                          Tiempo total: {formatElapsedTime(elapsedTime)}
                        </span>
                      )}
                    </div>
                  </div>
                ) : (
                  <>
                    <div style={resultSuccessStyles}>
                      <Icon name="check" size="md" color="var(--lumik-secondary, #e9c349)" />
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                        {result.successful > 0 && (
                          <span style={progressTextStyles}>
                            {result.successful} {result.successful === 1 ? 'fotografía importada' : 'fotografías importadas'}
                          </span>
                        )}
                        {result.videos_copied > 0 && (
                          <span style={progressTextStyles}>
                            {result.videos_copied} {result.videos_copied === 1 ? 'video copiado' : 'videos copiados'}
                          </span>
                        )}
                        {elapsedTime && (
                          <span style={progressCountStyles}>
                            Tiempo total: {formatElapsedTime(elapsedTime)}
                          </span>
                        )}
                      </div>
                    </div>
                    <div style={resultErrorStyles}>
                      <Icon name="x" size="md" color="var(--lumik-error, #ffb4ab)" />
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                        <span style={progressTextStyles}>
                          {result.failed} archivos fallaron
                        </span>
                        {result.failed_files.slice(0, 3).map((f: FailedFile) => (
                          <span key={f.path} style={progressCountStyles}>
                            {f.name}: {f.error}
                          </span>
                        ))}
                        {result.failed_files.length > 3 && (
                          <span style={progressCountStyles}>
                            y {result.failed_files.length - 3} más...
                          </span>
                        )}
                      </div>
                    </div>
                  </>
                )}
              </>
            )}

            {/* Import error */}
            {importError && (
              <div style={resultErrorStyles}>
                <Icon name="x" size="md" color="var(--lumik-error, #ffb4ab)" />
                <span style={progressTextStyles}>Error: {importError}</span>
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
          <span style={sectionTitleStyles}>Resumen de importación</span>
          <div style={summaryCardStyles}>
            {photoCount > 0 && (
              <div style={summaryRowStyles}>
                <span style={summaryLabelStyles}>Fotografías</span>
                <span style={summaryValueStyles}>{photoCount}</span>
              </div>
            )}
            {videoCount > 0 && (
              <div style={summaryRowStyles}>
                <span style={summaryLabelStyles}>Videos</span>
                <span style={summaryValueStyles}>{videoCount}</span>
              </div>
            )}
            <div style={summaryRowStyles}>
              <span style={summaryLabelStyles}>Tamaño total</span>
              <span style={summaryValueStyles}>{formatBytes(totalImportSize)}</span>
            </div>
            <div style={summaryRowStyles}>
              <span style={summaryLabelStyles}>Proyecto</span>
              <span style={summaryValueStyles}>
                {selectedProject?.name ?? initialProjectName ?? 'No seleccionado'}
              </span>
            </div>
            <div style={summaryRowStyles}>
              <span style={summaryLabelStyles}>Destino</span>
              <span style={summaryValueStyles}>
                {selectedDrive?.name ?? 'No seleccionado'}
              </span>
            </div>
            {selectedDrive && (
              <div style={summaryRowStyles}>
                <span style={summaryLabelStyles}>Espacio disponible</span>
                <span style={summaryValueStyles}>
                  {formatBytes(selectedDrive.totalBytes - selectedDrive.usedBytes)}
                </span>
              </div>
            )}
          </div>
        </div>

        <div style={validationCardStyles(true)}>
          <div style={validationIconStyles(true)}>
            <Icon
              name="check"
              size="md"
              color="var(--lumik-secondary, #e9c349)"
            />
          </div>
          <span style={validationTextStyles}>
            {settings?.convert_to_dng
              ? 'Todo listo. Los archivos se convertirán a DNG y se copiarán al disco de destino.'
              : 'Todo listo. Los archivos se copiarán al disco de destino sin conversión.'}
          </span>
        </div>
      </div>
    );
  };

  // Determine if we can proceed
  const canProceed = embedded
    ? currentStepIndex === 0 ? canProceedFromOrigin : true
    : currentStepIndex === 0
      ? canProceedFromOrigin
      : currentStepIndex === 1
        ? canProceedFromDestination
        : true;

  return (
    <div style={pageStyles}>
      <header style={headerStyles}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          {onClose && (
            <Button variant="ghost" size="sm" onClick={onClose} leftIcon={<Icon name="chevron-right" size="sm" style={{ transform: 'rotate(180deg)' }} />}>
              Volver
            </Button>
          )}
          <h1 style={titleStyles}>
            Importar fotos
            {initialProjectName && (
              <span style={{ fontWeight: 400, color: 'var(--lumik-on-surface-variant, #c2c6d7)', fontSize: '18px', marginLeft: '12px' }}>
                → {initialProjectName}
              </span>
            )}
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

      <footer style={footerStyles}>
        <div style={footerLeftStyles}>
          {importState.sourceFiles.length > 0 && (
            <span style={hintStyles}>
              {[
                photoCount > 0 && `${photoCount} ${photoCount === 1 ? 'fotografía' : 'fotografías'}`,
                videoCount > 0 && `${videoCount} ${videoCount === 1 ? 'video' : 'videos'}`,
              ].filter(Boolean).join(' • ')} • {formatBytes(totalImportSize)}
            </span>
          )}
        </div>
        <div style={footerRightStyles}>
          {currentStepIndex > 0 && !isImporting && !result && (
            <Button variant="secondary" onClick={handleBack}>
              <Icon name="chevron-right" size="sm" style={{ transform: 'rotate(180deg)' }} />
              Atrás
            </Button>
          )}
          {currentStepIndex < IMPORT_STEPS.length - 1 ? (
            <Button
              variant="primary"
              onClick={handleNext}
              disabled={!canProceed}
            >
              Siguiente
              <Icon name="chevron-right" size="sm" />
            </Button>
          ) : result ? (
            <Button variant="primary" onClick={embedded ? onClose : handleNewImport}>
              <Icon name={embedded ? 'check' : 'import'} size="sm" />
              {embedded ? 'Aceptar' : 'Nueva importación'}
            </Button>
          ) : (
            <Button
              variant="primary"
              onClick={handleStartImport}
              disabled={!canProceed || isImporting}
            >
              <Icon name="import" size="sm" />
              {isImporting ? 'Importando...' : 'Iniciar importación'}
            </Button>
          )}
        </div>
      </footer>

      <CreateProjectModal
        open={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        onSubmit={handleCreateProject}
        devices={devices ?? []}
        loading={isCreating}
        error={createError}
      />
    </div>
  );
}
