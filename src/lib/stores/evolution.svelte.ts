// Evolution Store - Evolution state & visualization
import type { Stage, EvolutionEvent, PersonalityVector } from '$lib/api/types';

function createEvolutionStore() {
    let currentStage = $state<Stage>('egg');
    let personality = $state<PersonalityVector | null>(null);
    let history = $state<EvolutionEvent[]>([]);
    let progress = $state(0);
    
    const stageOrder: Stage[] = ['egg', 'child', 'teen', 'adult', 'ultimate'];
    const stageIndex = $derived(stageOrder.indexOf(currentStage));
    const nextStage = $derived(stageOrder[stageIndex + 1] ?? null);
    const canEvolve = $derived(nextStage !== null);
    
    function loadFromBackend(data: { stage: Stage; personality: PersonalityVector; history: EvolutionEvent[]; progress: number }) {
        currentStage = data.stage;
        personality = data.personality;
        history = data.history;
        progress = data.progress;
    }
    
    function applyEvolution(event: EvolutionEvent) {
        currentStage = event.to_stage;
        personality = event.personality_vector;
        history = [event, ...history];
        progress = 0;
    }
    
    function updateProgress(p: number) { progress = Math.max(0, Math.min(1, p)); }
    
    return {
        get currentStage() { return currentStage; },
        get personality() { return personality; },
        get history() { return history; },
        get progress() { return progress; },
        get stageIndex() { return stageIndex; },
        get nextStage() { return nextStage; },
        get canEvolve() { return canEvolve; },
        loadFromBackend, applyEvolution, updateProgress,
    };
}

export const evolutionStore = createEvolutionStore();