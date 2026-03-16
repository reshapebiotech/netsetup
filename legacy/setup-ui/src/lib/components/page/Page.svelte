<script lang="ts">
  import Stepper from '$lib/components/stepper/Stepper.svelte';
  import Badge from '../badge/Badge.svelte';
  import Title from './Title.svelte';
  import Description from './Description.svelte';
  import RobotTag from '../robot-tag/RobotTag.svelte';
  import Section from './Section.svelte';

  const C = {
    Stepper,
    Badge,
    Title,
    Description,
    Section,
    RobotTag,
  };
</script>

<div class="flex flex-row grow h-screen justify-center pt-12">
  <div class="flex flex-col w-full sm:w-[640px] h-full gap-6 px-4 pt-16">
    <slot {C} />
  </div>
</div>
