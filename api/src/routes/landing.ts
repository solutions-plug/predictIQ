import { Router, Request, Response } from 'express';
import { AppError } from '../middleware/errorHandler';
import { queryDatabase } from '../config/database';

const router = Router();

router.post('/newsletter', async (req: Request, res: Response, next) => {
  try {
    const { email } = req.body;
    
    if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      throw new AppError('Invalid email address', 400);
    }

    await queryDatabase(
      'INSERT INTO newsletter_signups (email, created_at) VALUES ($1, NOW()) ON CONFLICT (email) DO NOTHING',
      [email]
    );

    res.status(201).json({
      status: 'success',
      message: 'Successfully subscribed to newsletter',
    });
  } catch (error) {
    next(error);
  }
});

router.get('/analytics', async (req: Request, res: Response, next) => {
  try {
    const result = await queryDatabase(
      'SELECT COUNT(*) as total_signups FROM newsletter_signups'
    );

    res.json({
      status: 'success',
      data: {
        totalSignups: parseInt(result.rows[0]?.total_signups || '0', 10),
      },
    });
  } catch (error) {
    next(error);
  }
});

export default router;
