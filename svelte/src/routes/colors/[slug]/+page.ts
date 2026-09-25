import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { colors } from '$lib/colors';

export const load: PageLoad = ({ params }) => {
  const color = colors.find((item) => item.slug === params.slug);

  if (!color) {
    error(404, 'Color not found');
  }

  return {
    color
  };
};