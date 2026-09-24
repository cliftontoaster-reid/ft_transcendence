export interface PlayerTrack {
  src: string;
  label: string;
  srclang: string;
  default?: boolean;
}

export interface VideoPlayerProps {
  /** URL du fichier vidéo (HLS, MP4, etc.) */
  src: string;
  /** Image de couverture/poster */
  poster?: string;
  /** Titre affiché dans l'overlay */
  title?: string;
  /** Sous-titres optionnels */
  tracks?: PlayerTrack[];
  /** Activer le mode automatique à la lecture */
  autoplay?: boolean;
  /** Action au clic sur le bouton Retour (si présent) */
  onBack?: () => void;
  /** Événement déclenché quand la vidéo se termine */
  onEnded?: () => void;
}