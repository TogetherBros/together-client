import { Character } from './types';

import dogImg from '../image/together_dog.png';
import catImg from '../image/together_cat.png';
import bunnyImg from '../image/together_bunny.png';
import bearImg from '../image/together_bear.png';
import birdImg from '../image/togegher_bird.png';
import beeImg from '../image/together_bee.png';
import logoImg from '../image/together_logo.png';

export { logoImg };

export const characterImages: Record<Character, string> = {
  DOG: dogImg,
  CAT: catImg,
  BUNNY: bunnyImg,
  BEAR: bearImg,
  BIRD: birdImg,
  BEE: beeImg,
};

export const characterMeta: { type: Character; label: string }[] = [
  { type: 'DOG', label: '강아지' },
  { type: 'CAT', label: '고양이' },
  { type: 'BUNNY', label: '토끼' },
  { type: 'BEAR', label: '곰' },
  { type: 'BIRD', label: '새' },
  { type: 'BEE', label: '꿀벌' },
];
