import numpy as np
from PIL import Image
import math

class PerlinNoise:
    def __init__(self, seed=0):
        np.random.seed(seed)
        # Generate permutation table
        self.p = np.arange(256)
        np.random.shuffle(self.p)
        self.p = np.tile(self.p, 2)  # Duplicate for easier indexing
        
        # Generate gradient vectors
        self.gradients = []
        for i in range(256):
            angle = np.random.uniform(0, 2 * np.pi)
            self.gradients.append([np.cos(angle), np.sin(angle)])
        self.gradients = np.array(self.gradients)
    
    def fade(self, t):
        """Fade function for smooth interpolation"""
        return t * t * t * (t * (t * 6 - 15) + 10)
    
    def lerp(self, t, a, b):
        """Linear interpolation"""
        return a + t * (b - a)
    
    def grad(self, hash_val, x, y):
        """Gradient function"""
        grad_vec = self.gradients[hash_val & 255]
        return grad_vec[0] * x + grad_vec[1] * y
    
    def noise2d(self, x, y, period_x=1, period_y=None):
        """Generate 2D Perlin noise with optional periodicity"""
        if period_x is not None:
            x = x % period_x
        if period_y is not None:
            y = y % period_y
            
        # Grid coordinates
        xi = int(x) & 255
        yi = int(y) & 255
        
        # Fractional part
        xf = x - int(x)
        yf = y - int(y)
        
        # Fade curves
        u = self.fade(xf)
        v = self.fade(yf)
        
        # Hash coordinates
        aa = self.p[self.p[xi] + yi]
        ab = self.p[self.p[xi] + yi + 1]
        ba = self.p[self.p[xi + 1] + yi]
        bb = self.p[self.p[xi + 1] + yi + 1]
        
        # Gradient calculations
        g1 = self.grad(aa, xf, yf)
        g2 = self.grad(ba, xf - 1, yf)
        g3 = self.grad(ab, xf, yf - 1)
        g4 = self.grad(bb, xf - 1, yf - 1)
        
        # Interpolate
        x1 = self.lerp(u, g1, g2)
        x2 = self.lerp(u, g3, g4)
        
        return self.lerp(v, x1, x2)

def load_and_process_alpha_image(image_path, nx, ny):
    """
    Load an image, resize it to match output resolution, and convert to grayscale values (0-1)
    """
    try:
        # Load the image
        alpha_img = Image.open(image_path)
        print(f"Loaded grayscale image: {image_path} ({alpha_img.size[0]}x{alpha_img.size[1]})")
        
        # Convert to grayscale if it's not already
        if alpha_img.mode != 'L':
            alpha_img = alpha_img.convert('L')
        
        # Resize to match output resolution
        # Using LANCZOS for high-quality resampling
        alpha_img = alpha_img.resize((nx, ny), Image.Resampling.LANCZOS)
        
        # Convert to numpy array and normalize to 0-1 range
        alpha_array = np.array(alpha_img, dtype=np.float32) / 255.0
        
        print(f"Resized grayscale image to {nx}x{ny}")
        print(f"Image values range: {np.min(alpha_array):.3f} to {np.max(alpha_array):.3f}")
        return alpha_array
        
    except Exception as e:
        print(f"Error loading grayscale image: {e}")
        print("Falling back to Perlin noise for grayscale channel")
        return None

def normalize_channel_data(channel_data, channel_name=""):
    """
    Properly normalize channel data to 0-1 range with debugging info
    """
    min_val = np.min(channel_data)
    max_val = np.max(channel_data)
    
    print(f"{channel_name} raw range: {min_val:.3f} to {max_val:.3f}")
    
    if max_val > min_val:
        normalized = (channel_data - min_val) / (max_val - min_val)
    else:
        print(f"Warning: {channel_name} has constant values, setting to 0.5")
        normalized = np.full_like(channel_data, 0.5)
    
    print(f"{channel_name} normalized range: {np.min(normalized):.3f} to {np.max(normalized):.3f}")
    return normalized

def generate_sphere_texture(seed, nx, ny, output_filename="sphere_texture.png", alpha_image_path=None):
    """
    Generate a texture with Perlin noises for RGB channels and either loaded image or 
    Perlin noise for the 4th channel (as grayscale values, not alpha transparency).
    
    Args:
        seed: Random seed for Perlin noise
        nx: Texture width
        ny: Texture height
        output_filename: Output file name
        alpha_image_path: Path to image for 4th channel (optional)
    """
    # Create noise generator
    noise = PerlinNoise(seed)
    
    # Initialize RGBA arrays
    rgba = np.zeros((ny, nx, 4), dtype=np.uint8)
    
    # Try to load image for 4th channel if provided
    fourth_channel_data = None
    if alpha_image_path:
        fourth_channel_data = load_and_process_alpha_image(alpha_image_path, nx, ny)
    
    # Parameters for different noise layers (RGB channels only)
    noise_params = [
        {"scale": 24.0, "octaves": 4, "persistence": 0.5},   # Red channel
        {"scale": 12.0, "octaves": 3, "persistence": 0.6},  # Green channel
        {"scale": 6.0, "octaves": 5, "persistence": 0.4},   # Blue channel
    ]
    
    channel_names = ['Red', 'Green', 'Blue']
    
    # Generate noise for RGB channels (0, 1, 2)
    for channel in range(3):
        params = noise_params[channel]
        channel_data = np.zeros((ny, nx))
        
        # Generate multiple octaves
        for octave in range(params["octaves"]):
            frequency = params["scale"] * (2 ** octave)
            amplitude = params["persistence"] ** octave
            
            # For sphere mapping, we need to ensure periodicity
            # The texture wraps around horizontally (longitude) but not vertically (latitude)
            for y in range(ny):
                for x in range(nx):
                    # Map to sphere coordinates
                    # x corresponds to longitude (0 to 2π) - needs to be periodic
                    # y corresponds to latitude (0 to π) - no periodicity needed
                    
                    # Normalize coordinates
                    norm_x = x / nx
                    norm_y = y / ny
                    
                    # Scale by frequency
                    sample_x = norm_x * frequency
                    sample_y = norm_y * frequency
                    
                    # Generate noise with horizontal periodicity
                    noise_val = noise.noise2d(
                        sample_x, 
                        sample_y, 
                        period_x=frequency,  # Horizontal periodicity
                        period_y=None        # No vertical periodicity
                    )
                    
                    channel_data[y, x] += noise_val * amplitude
        
        # Normalize to 0-1 range with debugging
        channel_data = normalize_channel_data(channel_data, channel_names[channel])
        
        # Apply different transformations for each channel
        if channel == 0:  # Red - high contrast
            channel_data = np.power(channel_data, 0.7)
        elif channel == 1:  # Green - smoother
            channel_data = np.sin(channel_data * np.pi) ** 2
        elif channel == 2:  # Blue - inverse
            channel_data = 1.0 - channel_data
        
        # Convert to 0-255 range
        rgba[:, :, channel] = (channel_data * 255).astype(np.uint8)
    
    # Handle 4th channel (stored in alpha position but represents grayscale values)
    if fourth_channel_data is not None:
        # Use loaded image data for 4th channel as grayscale values
        rgba[:, :, 3] = (fourth_channel_data * 255).astype(np.uint8)
        print("Using loaded image for 4th channel (grayscale values)")
    else:
        # Fall back to Perlin noise for 4th channel
        print("Using Perlin noise for 4th channel (grayscale values)")
        fourth_params = {"scale": 4.0, "octaves": 3, "persistence": 0.7}
        fourth_channel_data = np.zeros((ny, nx))
        
        for octave in range(fourth_params["octaves"]):
            frequency = fourth_params["scale"] * (2 ** octave)
            amplitude = fourth_params["persistence"] ** octave
            
            for y in range(ny):
                for x in range(nx):
                    norm_x = x / nx
                    norm_y = y / ny
                    sample_x = norm_x * frequency
                    sample_y = norm_y * frequency
                    
                    noise_val = noise.noise2d(
                        sample_x, 
                        sample_y, 
                        period_x=frequency,
                        period_y=None
                    )
                    
                    fourth_channel_data[y, x] += noise_val * amplitude
        
        # Normalize 4th channel to full 0-1 range with debugging
        fourth_channel_data = normalize_channel_data(fourth_channel_data, "Grayscale (4th)")
        
        # Ensure we have a good distribution - no additional transformations
        # to keep the full 0-1 range
        
        # Convert to 0-255 range (full grayscale range)
        rgba[:, :, 3] = (fourth_channel_data * 255).astype(np.uint8)
        
        print(f"Final 4th channel range: {np.min(rgba[:, :, 3])} to {np.max(rgba[:, :, 3])}")
    # APPLY FLIPS TO THE FINAL RESULT
    # Flip vertically (top/bottom)
    rgba = np.flipud(rgba)
    # Flip horizontally (left/right) 
    # rgba = np.fliplr(rgba)
    # Create and save image
    image = Image.fromarray(rgba, 'RGBA')
    image.save(output_filename)
    
    print(f"Generated {nx}x{ny} sphere texture: {output_filename}")
    print(f"Note: 4th channel contains grayscale values, not alpha transparency")
    print(f"Used seed: {seed}")
    
    return rgba

def main():
    # Parameters
    seed = 42

    ny = int(432)  # Height (latitude)
    nx = int(2 * ny) # Width (longitude)
    
    # Optional: Path to image for 4th channel (grayscale values)
    # Set to None to use Perlin noise, or provide a path to use an image
    fourth_channel_image_path = "mask.jpg"  # Example: "grayscale_mask.png"
    
    # You can uncomment and modify the line below to use an image for 4th channel:
    # fourth_channel_image_path = "your_grayscale_image.png"
    
    # Generate the texture
    texture = generate_sphere_texture(
        seed, 
        nx, 
        ny, 
        "sphere_texture.png", 
        alpha_image_path=fourth_channel_image_path
    )
    
    # Generate individual channel previews
    channel_names = ['Red', 'Green', 'Blue', 'Grayscale']
    for i, channel_name in enumerate(channel_names):
        if i < 3:  # RGB channels - show colored preview
            channel_img = np.zeros((ny, nx, 3), dtype=np.uint8)
            channel_img[:, :, i] = texture[:, :, i]
            
            # Convert to RGB and save
            preview_img = Image.fromarray(channel_img, 'RGB')
            preview_img.save(f"channel_{channel_name.lower()}_preview.png")
        else:  # 4th channel - show as grayscale image
            # Create a grayscale image directly from the 4th channel data
            grayscale_data = texture[:, :, 3]
            
            # Debug: print some statistics about the grayscale data
            print(f"Grayscale preview data range: {np.min(grayscale_data)} to {np.max(grayscale_data)}")
            print(f"Grayscale preview mean: {np.mean(grayscale_data):.1f}")
            
            # Save as grayscale image
            preview_img = Image.fromarray(grayscale_data, 'L')
            preview_img.save(f"channel_{channel_name.lower()}_preview.png")
        
        print(f"Generated {channel_name} channel preview")

if __name__ == "__main__":
    main()
