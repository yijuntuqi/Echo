import type { EvolutionRecord, Stage } from '$lib/api/types';

const ORDER: Stage[] = ['egg', 'child', 'teen', 'adult', 'ultimate'];

export const STAGE_LABELS: Record<Stage, string> = {
  egg: '蛋',
  child: '幼年',
  teen: '少年',
  adult: '成年',
  ultimate: '究极',
};

function createEvolutionStore() {
  let stage = $state<Stage>('egg');
  /** 32 dimensions, each in [-1, 1]. */
  let personality = $state<number[]>(Array(32).fill(0));
  let history = $state<EvolutionRecord[]>([]);
  let progress = $state(0);

  const stageIndex = $derived(ORDER.indexOf(stage));
  const nextStage = $derived<Stage | null>(ORDER[stageIndex + 1] ?? null);
  const canEvolve = $derived(nextStage !== null);
  const isFinalStage = $derived(nextStage === null);

  function loadFromBackend(state: {
    stage: Stage;
    personality: number[];
    history: EvolutionRecord[];
    progress: number;
  }): void {
    stage = state.stage;
    personality = state.personality;
    history = state.history;
    progress = state.progress;
  }

  /** Apply an evolution event pushed from the backend. */
  function applyEvolution(record: EvolutionRecord): void {
    stage = record.to_stage;
    if (record.personality_vector?.length) personality = record.personality_vector;
    history = [record, ...history];
    progress = 0;
  }

  function setProgress(p: number): void {
    progress = Math.min(1, Math.max(0, p));
  }

  return {
    get stage() { return stage; },
    get stageLabel() { return STAGE_LABELS[stage]; },
    get nextStage() { return nextStage; },
    get nextStageLabel() { return nextStage ? STAGE_LABELS[nextStage] : null; },
    get stageIndex() { return stageIndex; },
    get canEvolve() { return canEvolve; },
    get isFinalStage() { return isFinalStage; },
    get personality() { return personality; },
    get history() { return history; },
    get progress() { return progress; },
    loadFromBackend,
    applyEvolution,
    setProgress,
  };
}

export const evolutionStore = createEvolutionStore();
