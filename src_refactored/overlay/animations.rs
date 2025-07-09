// Animation system for overlay elements
// Custom animation framework with easing functions

use super::{OverlayElement, Color, Animation, AnimationType};
use crate::utils::geometry::{Point, Rectangle};
use std::time::{Duration, Instant};

pub struct AnimationManager {
    animations: Vec<AnimationInstance>,
    next_id: u64,
}

#[derive(Debug, Clone)]
pub struct AnimationInstance {
    pub id: u64,
    pub element_id: String,
    pub animation: Animation,
    pub easing: EasingFunction,
    pub auto_reverse: bool,
    pub repeat_count: Option<u32>,
    pub current_repeat: u32,
    pub is_reversed: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
    Back,
    Sine,
    Quad,
    Cubic,
    Quart,
    Quint,
    Expo,
    Circ,
}

pub struct AnimationBuilder {
    animation_type: AnimationType,
    duration: Duration,
    easing: EasingFunction,
    delay: Duration,
    auto_reverse: bool,
    repeat_count: Option<u32>,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
            next_id: 0,
        }
    }

    pub fn add_animation(&mut self, element_id: String, animation: Animation, easing: EasingFunction) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let instance = AnimationInstance {
            id,
            element_id,
            animation,
            easing,
            auto_reverse: false,
            repeat_count: None,
            current_repeat: 0,
            is_reversed: false,
        };

        self.animations.push(instance);
        id
    }

    pub fn add_animation_with_options(
        &mut self,
        element_id: String,
        animation: Animation,
        easing: EasingFunction,
        auto_reverse: bool,
        repeat_count: Option<u32>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let instance = AnimationInstance {
            id,
            element_id,
            animation,
            easing,
            auto_reverse,
            repeat_count,
            current_repeat: 0,
            is_reversed: false,
        };

        self.animations.push(instance);
        id
    }

    pub fn remove_animation(&mut self, id: u64) {
        self.animations.retain(|anim| anim.id != id);
    }

    pub fn remove_animations_for_element(&mut self, element_id: &str) {
        self.animations.retain(|anim| anim.element_id != element_id);
    }

    pub fn update_animations(&mut self, delta_time: Duration) {
        let current_time = Instant::now();
        let mut finished_animations = Vec::new();

        for animation in &mut self.animations {
            // Update the base animation
            animation.animation.update(delta_time);

            // Apply easing function
            let raw_progress = animation.animation.progress;
            let eased_progress = if animation.is_reversed {
                1.0 - apply_easing(1.0 - raw_progress, animation.easing)
            } else {
                apply_easing(raw_progress, animation.easing)
            };

            // Temporarily set the eased progress for element application
            let original_progress = animation.animation.progress;
            animation.animation.progress = eased_progress;

            // Check if animation is finished
            if animation.animation.is_finished(current_time) {
                if animation.auto_reverse && !animation.is_reversed {
                    // Start reverse animation
                    animation.is_reversed = true;
                    animation.animation.start_time = current_time;
                    animation.animation.progress = 0.0;
                } else if let Some(repeat_count) = animation.repeat_count {
                    if animation.current_repeat < repeat_count - 1 {
                        // Restart animation
                        animation.current_repeat += 1;
                        animation.is_reversed = false;
                        animation.animation.start_time = current_time;
                        animation.animation.progress = 0.0;
                    } else {
                        finished_animations.push(animation.id);
                    }
                } else {
                    finished_animations.push(animation.id);
                }
            }

            // Restore original progress
            animation.animation.progress = original_progress;
        }

        // Remove finished animations
        for id in finished_animations {
            self.remove_animation(id);
        }
    }

    pub fn apply_animations_to_element(&self, element: &mut OverlayElement) {
        for animation in &self.animations {
            if animation.element_id == element.id {
                // Apply easing to the animation
                let raw_progress = animation.animation.progress;
                let eased_progress = if animation.is_reversed {
                    1.0 - apply_easing(1.0 - raw_progress, animation.easing)
                } else {
                    apply_easing(raw_progress, animation.easing)
                };

                // Create a temporary animation with eased progress
                let mut temp_animation = animation.animation.clone();
                temp_animation.progress = eased_progress;
                temp_animation.apply_to_element(element);
            }
        }
    }

    pub fn get_animation_count(&self) -> usize {
        self.animations.len()
    }

    pub fn get_animations_for_element(&self, element_id: &str) -> Vec<&AnimationInstance> {
        self.animations
            .iter()
            .filter(|anim| anim.element_id == element_id)
            .collect()
    }

    pub fn is_element_animating(&self, element_id: &str) -> bool {
        self.animations.iter().any(|anim| anim.element_id == element_id)
    }

    pub fn pause_all_animations(&mut self) {
        // In a more complete implementation, you'd add pause/resume functionality
        // For now, we'll just clear all animations
        self.animations.clear();
    }

    pub fn clear_all_animations(&mut self) {
        self.animations.clear();
    }
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationBuilder {
    pub fn new(animation_type: AnimationType, duration: Duration) -> Self {
        Self {
            animation_type,
            duration,
            easing: EasingFunction::Linear,
            delay: Duration::from_millis(0),
            auto_reverse: false,
            repeat_count: None,
        }
    }

    pub fn with_easing(mut self, easing: EasingFunction) -> Self {
        self.easing = easing;
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn with_auto_reverse(mut self, auto_reverse: bool) -> Self {
        self.auto_reverse = auto_reverse;
        self
    }

    pub fn with_repeat_count(mut self, count: u32) -> Self {
        self.repeat_count = Some(count);
        self
    }

    pub fn build(self) -> (Animation, EasingFunction, bool, Option<u32>) {
        let animation = Animation::new(
            self.animation_type,
            self.duration,
            Instant::now() + self.delay,
        );
        
        (animation, self.easing, self.auto_reverse, self.repeat_count)
    }
}

// Easing function implementations
fn apply_easing(t: f64, easing: EasingFunction) -> f64 {
    let t = t.clamp(0.0, 1.0);
    
    match easing {
        EasingFunction::Linear => t,
        EasingFunction::EaseIn => ease_in_quad(t),
        EasingFunction::EaseOut => ease_out_quad(t),
        EasingFunction::EaseInOut => ease_in_out_quad(t),
        EasingFunction::Bounce => ease_out_bounce(t),
        EasingFunction::Elastic => ease_out_elastic(t),
        EasingFunction::Back => ease_out_back(t),
        EasingFunction::Sine => ease_in_out_sine(t),
        EasingFunction::Quad => ease_in_out_quad(t),
        EasingFunction::Cubic => ease_in_out_cubic(t),
        EasingFunction::Quart => ease_in_out_quart(t),
        EasingFunction::Quint => ease_in_out_quint(t),
        EasingFunction::Expo => ease_in_out_expo(t),
        EasingFunction::Circ => ease_in_out_circ(t),
    }
}

fn ease_in_quad(t: f64) -> f64 {
    t * t
}

fn ease_out_quad(t: f64) -> f64 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_in_out_quad(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn ease_in_out_quart(t: f64) -> f64 {
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
    }
}

fn ease_in_out_quint(t: f64) -> f64 {
    if t < 0.5 {
        16.0 * t * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
    }
}

fn ease_in_out_sine(t: f64) -> f64 {
    -(std::f64::consts::PI * t).cos() / 2.0 + 0.5
}

fn ease_in_out_expo(t: f64) -> f64 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else if t < 0.5 {
        (20.0 * t - 10.0).exp2() / 2.0
    } else {
        (2.0 - (-20.0 * t + 10.0).exp2()) / 2.0
    }
}

fn ease_in_out_circ(t: f64) -> f64 {
    if t < 0.5 {
        (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
    } else {
        ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
    }
}

fn ease_out_bounce(t: f64) -> f64 {
    const N1: f64 = 7.5625;
    const D1: f64 = 2.75;

    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

fn ease_out_elastic(t: f64) -> f64 {
    const C4: f64 = 2.0 * std::f64::consts::PI / 3.0;

    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        (-10.0 * t).exp2() * (10.0 * t - 0.75) * C4.sin() + 1.0
    }
}

fn ease_out_back(t: f64) -> f64 {
    const C1: f64 = 1.70158;
    const C3: f64 = C1 + 1.0;

    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

// Preset animation creators
pub fn create_fade_in(duration: Duration) -> AnimationBuilder {
    AnimationBuilder::new(AnimationType::FadeIn, duration)
}

pub fn create_fade_out(duration: Duration) -> AnimationBuilder {
    AnimationBuilder::new(AnimationType::FadeOut, duration)
}

pub fn create_scale_animation(from_scale: f64, to_scale: f64, duration: Duration) -> AnimationBuilder {
    AnimationBuilder::new(AnimationType::Scale(from_scale, to_scale), duration)
}

pub fn create_move_animation(from_pos: Point, to_pos: Point, duration: Duration) -> AnimationBuilder {
    AnimationBuilder::new(AnimationType::Move(from_pos, to_pos), duration)
}

pub fn create_pulse_animation(duration: Duration) -> AnimationBuilder {
    AnimationBuilder::new(AnimationType::Pulse, duration)
        .with_repeat_count(u32::MAX) // Infinite repeat
}

// Animation sequence builder
pub struct AnimationSequence {
    steps: Vec<AnimationStep>,
    current_step: usize,
    total_duration: Duration,
}

#[derive(Debug, Clone)]
struct AnimationStep {
    animation: Animation,
    easing: EasingFunction,
    delay: Duration,
    duration: Duration,
}

impl AnimationSequence {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            current_step: 0,
            total_duration: Duration::from_millis(0),
        }
    }

    pub fn add_step(mut self, animation: Animation, easing: EasingFunction, delay: Duration) -> Self {
        let step = AnimationStep {
            duration: animation.duration,
            animation,
            easing,
            delay,
        };
        
        self.total_duration += step.duration + step.delay;
        self.steps.push(step);
        self
    }

    pub fn get_total_duration(&self) -> Duration {
        self.total_duration
    }

    pub fn get_current_animation(&self, elapsed_time: Duration) -> Option<(&Animation, EasingFunction)> {
        let mut current_time = Duration::from_millis(0);
        
        for step in &self.steps {
            if elapsed_time >= current_time && elapsed_time < current_time + step.delay + step.duration {
                if elapsed_time >= current_time + step.delay {
                    return Some((&step.animation, step.easing));
                }
            }
            current_time += step.delay + step.duration;
        }
        
        None
    }
}

impl Default for AnimationSequence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_manager_creation() {
        let manager = AnimationManager::new();
        assert_eq!(manager.get_animation_count(), 0);
    }

    #[test]
    fn test_add_animation() {
        let mut manager = AnimationManager::new();
        let animation = Animation::new(
            AnimationType::FadeIn,
            Duration::from_millis(1000),
            Instant::now(),
        );
        
        let id = manager.add_animation(
            "test_element".to_string(),
            animation,
            EasingFunction::Linear,
        );
        
        assert_eq!(manager.get_animation_count(), 1);
        assert!(manager.is_element_animating("test_element"));
        assert!(!manager.is_element_animating("other_element"));
        
        manager.remove_animation(id);
        assert_eq!(manager.get_animation_count(), 0);
    }

    #[test]
    fn test_animation_builder() {
        let builder = AnimationBuilder::new(AnimationType::FadeIn, Duration::from_millis(500))
            .with_easing(EasingFunction::EaseInOut)
            .with_delay(Duration::from_millis(100))
            .with_auto_reverse(true)
            .with_repeat_count(3);

        let (animation, easing, auto_reverse, repeat_count) = builder.build();
        
        assert_eq!(animation.duration, Duration::from_millis(500));
        assert!(matches!(easing, EasingFunction::EaseInOut));
        assert!(auto_reverse);
        assert_eq!(repeat_count, Some(3));
    }

    #[test]
    fn test_easing_functions() {
        // Test that easing functions return values in [0, 1] range
        let test_values = [0.0, 0.25, 0.5, 0.75, 1.0];
        let easing_functions = [
            EasingFunction::Linear,
            EasingFunction::EaseIn,
            EasingFunction::EaseOut,
            EasingFunction::EaseInOut,
            EasingFunction::Bounce,
            EasingFunction::Elastic,
            EasingFunction::Back,
            EasingFunction::Sine,
            EasingFunction::Quad,
            EasingFunction::Cubic,
        ];

        for easing in easing_functions {
            for &t in &test_values {
                let result = apply_easing(t, easing);
                assert!(result >= 0.0 && result <= 1.0 || (easing == EasingFunction::Back && result >= -0.1), 
                       "Easing {:?} at t={} returned {}", easing, t, result);
            }
        }
    }

    #[test]
    fn test_linear_easing() {
        assert_eq!(apply_easing(0.0, EasingFunction::Linear), 0.0);
        assert_eq!(apply_easing(0.5, EasingFunction::Linear), 0.5);
        assert_eq!(apply_easing(1.0, EasingFunction::Linear), 1.0);
    }

    #[test]
    fn test_ease_in_quad() {
        assert_eq!(ease_in_quad(0.0), 0.0);
        assert_eq!(ease_in_quad(0.5), 0.25);
        assert_eq!(ease_in_quad(1.0), 1.0);
    }

    #[test]
    fn test_ease_out_quad() {
        assert_eq!(ease_out_quad(0.0), 0.0);
        assert_eq!(ease_out_quad(1.0), 1.0);
        // At t=0.5, ease_out_quad should be > 0.5 (faster initial progress)
        assert!(ease_out_quad(0.5) > 0.5);
    }

    #[test]
    fn test_animation_sequence() {
        let sequence = AnimationSequence::new()
            .add_step(
                Animation::new(AnimationType::FadeIn, Duration::from_millis(100), Instant::now()),
                EasingFunction::Linear,
                Duration::from_millis(0),
            )
            .add_step(
                Animation::new(AnimationType::FadeOut, Duration::from_millis(200), Instant::now()),
                EasingFunction::EaseOut,
                Duration::from_millis(50),
            );

        assert_eq!(sequence.get_total_duration(), Duration::from_millis(350)); // 100 + 200 + 50
        assert_eq!(sequence.steps.len(), 2);
    }

    #[test]
    fn test_preset_animations() {
        let fade_in = create_fade_in(Duration::from_millis(300));
        let (animation, easing, _, _) = fade_in.build();
        
        assert!(matches!(animation.animation_type, AnimationType::FadeIn));
        assert_eq!(animation.duration, Duration::from_millis(300));
        assert!(matches!(easing, EasingFunction::Linear));
    }

    #[test]
    fn test_remove_animations_for_element() {
        let mut manager = AnimationManager::new();
        
        // Add animations for multiple elements
        let animation1 = Animation::new(AnimationType::FadeIn, Duration::from_millis(1000), Instant::now());
        let animation2 = Animation::new(AnimationType::FadeOut, Duration::from_millis(1000), Instant::now());
        let animation3 = Animation::new(AnimationType::Pulse, Duration::from_millis(1000), Instant::now());
        
        manager.add_animation("element1".to_string(), animation1, EasingFunction::Linear);
        manager.add_animation("element2".to_string(), animation2, EasingFunction::Linear);
        manager.add_animation("element1".to_string(), animation3, EasingFunction::Linear);
        
        assert_eq!(manager.get_animation_count(), 3);
        assert_eq!(manager.get_animations_for_element("element1").len(), 2);
        assert_eq!(manager.get_animations_for_element("element2").len(), 1);
        
        // Remove animations for element1
        manager.remove_animations_for_element("element1");
        
        assert_eq!(manager.get_animation_count(), 1);
        assert_eq!(manager.get_animations_for_element("element1").len(), 0);
        assert_eq!(manager.get_animations_for_element("element2").len(), 1);
    }
}