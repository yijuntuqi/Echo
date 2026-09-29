// Pet Store - Core pet state management
import { browser } from '$app/environment';

export type Stage = 'egg' | 'child' | 'teen' | 'adult' | 'ultimate';
export type PetAnimationState = 'idle' | 'walk' | 'sleep' | 'talk' | 'react' | 'evolve';
export interface Vec2 { x: number; y: number; }

function createPetStore() {
    let stage = $state<Stage>('egg');
    let animation = $state<PetAnimationState>('idle');
    let position = $state<Vec2>({ x: 100, y: 100 });
    let isVisible = $state(true);
    let clickThrough = $state(true);
    let personality = $state<Record<string, number>>({});
    let lastInteraction = $state<Date | null>(null);
    
    const assetPrefix = $derived(`/${stage}`);
    const svgPaths = $derived({
        idle: `${assetPrefix}/idle.svg`,
        walk: `${assetPrefix}/walk.svg`,
        sleep: `${assetPrefix}/sleep.svg`,
        talk: `${assetPrefix}/talk.svg`,
        react: `${assetPrefix}/react.svg`,
        evolve: `${assetPrefix}/evolve.svg`,
    });
    
    function setStage(newStage: Stage) {
        stage = newStage;
        playAnimation('evolve');
    }
    
    function playAnimation(anim: PetAnimationState, duration = 2000) {
        animation = anim;
        if (anim !== 'idle' && anim !== 'sleep') {
            setTimeout(() => { animation = 'idle'; }, duration);
        }
    }
    
    function setPosition(pos: Vec2) { position = pos; }
    function toggleVisibility() { isVisible = !isVisible; }
    function setClickThrough(enabled: boolean) { clickThrough = enabled; }
    function updatePersonality(vec: Record<string, number>) { personality = vec; }
    function touch() { lastInteraction = new Date(); }
    
    if (browser) {
        const saved = localStorage.getItem('pet:position');
        if (saved) { try { position = JSON.parse(saved); } catch {} }
        $effect(() => { localStorage.setItem('pet:position', JSON.stringify(position)); });
    }
    
    return {
        get stage() { return stage; },
        get animation() { return animation; },
        get position() { return position; },
        get isVisible() { return isVisible; },
        get clickThrough() { return clickThrough; },
        get personality() { return personality; },
        get lastInteraction() { return lastInteraction; },
        get svgPaths() { return svgPaths; },
        setStage, playAnimation, setPosition, toggleVisibility, setClickThrough, updatePersonality, touch,
    };
}

export const petStore = createPetStore();