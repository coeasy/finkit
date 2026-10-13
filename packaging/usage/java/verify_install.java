import com.finkit.Indicators;

public final class verify_install {
    public static void main(String[] args) {
        double[] result = Indicators.sma(new double[] {1, 2, 3, 4}, 3);
        if (result.length != 4 || !Double.isNaN(result[0]) || !Double.isNaN(result[1])
                || result[2] != 2.0 || result[3] != 3.0) {
            throw new AssertionError(java.util.Arrays.toString(result));
        }
        System.out.println("installed Java JAR SMA smoke passed");
    }
}
