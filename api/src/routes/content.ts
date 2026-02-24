import { Router, Response } from 'express';
import { authenticate, AuthRequest } from '../middleware/auth';
import { ContentService } from '../services/contentService';

const router = Router();
const contentService = new ContentService();

// GET /api/v1/content/:section - Public endpoint
router.get('/:section', async (req, res: Response, next) => {
  try {
    const { section } = req.params;
    const content = await contentService.getContent(section);

    if (!content) {
      return res.status(404).json({ error: 'Content not found' });
    }

    res.json({
      section: content.section,
      ...content.content,
      version: content.version,
      lastUpdated: content.created_at,
    });
  } catch (error) {
    next(error);
  }
});

// POST /api/v1/content/:section - Admin only
router.post('/:section', authenticate, async (req: AuthRequest, res: Response, next) => {
  try {
    const { section } = req.params;
    const content = req.body;

    // Validate content
    const validation = contentService.validateContent(section, content);
    if (!validation.valid) {
      return res.status(400).json({ error: 'Validation failed', details: validation.errors });
    }

    const updated = await contentService.updateContent(section, content, req.user!.id);

    res.json({
      section: updated.section,
      ...updated.content,
      version: updated.version,
      lastUpdated: updated.created_at,
    });
  } catch (error) {
    next(error);
  }
});

// GET /api/v1/content/:section/versions - Admin only
router.get('/:section/versions', authenticate, async (req: AuthRequest, res: Response, next) => {
  try {
    const { section } = req.params;
    const limit = parseInt(req.query.limit as string) || 10;

    const versions = await contentService.getVersionHistory(section, limit);

    res.json({
      section,
      versions: versions.map(v => ({
        version: v.version,
        content: v.content,
        createdBy: v.created_by,
        createdAt: v.created_at,
      })),
    });
  } catch (error) {
    next(error);
  }
});

// GET /api/v1/content/:section/versions/:version - Admin only
router.get('/:section/versions/:version', authenticate, async (req: AuthRequest, res: Response, next) => {
  try {
    const { section, version } = req.params;
    const content = await contentService.getVersion(section, parseInt(version));

    if (!content) {
      return res.status(404).json({ error: 'Version not found' });
    }

    res.json({
      section: content.section,
      ...content.content,
      version: content.version,
      createdAt: content.created_at,
    });
  } catch (error) {
    next(error);
  }
});

// POST /api/v1/content/:section/preview - Admin only
router.post('/:section/preview', authenticate, async (req: AuthRequest, res: Response, next) => {
  try {
    const { section } = req.params;
    const content = req.body;

    const preview = await contentService.previewContent(section, content);

    res.json({
      section,
      preview,
    });
  } catch (error) {
    next(error);
  }
});

export default router;
